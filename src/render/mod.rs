use crate::dbus::screencasting::pw_utils::CastTarget;
use std::borrow::Cow;
use std::time::Duration;

pub(crate) mod decorations;
pub(crate) mod screenshot_ui;

use smithay::{
    backend::renderer::{
        element::{
            Element, Kind, RenderElement, memory::MemoryRenderBufferRenderElement, render_elements,
            texture::TextureRenderElement, utils::RescaleRenderElement,
        },
        gles::{
            GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
            UniformType, element::PixelShaderElement,
        },
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    input::pointer::{CursorImageStatus, CursorImageSurfaceData},
    output::Output,
    utils::{Logical, Physical, Point, Rectangle, Scale, Transform},
};

use smithay::backend::renderer::element::AsRenderElements;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::desktop::layer_map_for_output;
use smithay::wayland::shell::wlr_layer::Layer as WlrLayer;

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::utils::Size;

use smithay::reexports::wayland_server::Resource;
use smithay::utils::IsAlive;
use smithay::wayland::compositor::with_states;
use smithay::wayland::seat::WaylandFocus;

use srwc::canvas::{self, CanvasPos, canvas_to_screen};

mod background;
mod blur;
pub mod dmabuf;
mod screencopy;

pub use background::{init_background, update_background_element};
pub use blur::{
    BlurCache, BlurLayer, BlurRequestData, compile_blur_shaders, process_blur_requests,
};
pub use screencopy::{render_capture_frames, render_screencopy};

render_elements! {
    pub OutputRenderElements<=GlesRenderer>;
    Background=RescaleRenderElement<PixelShaderElement>,
    Decoration=RescaleRenderElement<MemoryRenderBufferRenderElement<GlesRenderer>>,
    Window=RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>,
    CsdWindow=RescaleRenderElement<RoundedCornerElement>,
    Layer=WaylandSurfaceRenderElement<GlesRenderer>,
    Cursor=MemoryRenderBufferRenderElement<GlesRenderer>,
    CursorSurface=smithay::backend::renderer::element::Wrap<WaylandSurfaceRenderElement<GlesRenderer>>,
    Blur=TextureRenderElement<GlesTexture>,
}

// Shadow and Decoration share inner types with Background.
// We can't add them to render_elements! because it generates conflicting From impls.
// Instead we construct them directly using the existing Background variant.
// Helpers below create the elements and wrap them in the correct variant.

/// Uniform declarations for background shaders.
/// Shaders receive only u_camera — zoom is handled externally via RescaleRenderElement.
pub const BG_UNIFORMS: &[UniformName<'static>] = &[UniformName {
    name: std::borrow::Cow::Borrowed("u_camera"),
    type_: UniformType::_2f,
}];

/// Shadow shader source — soft box-shadow around SSD windows.
const SHADOW_SHADER_SRC: &str = include_str!("shaders/shadow.glsl");

/// Uniform declarations for the shadow shader.
pub const SHADOW_UNIFORMS: &[UniformName<'static>] = &[
    UniformName {
        name: std::borrow::Cow::Borrowed("u_window_rect"),
        type_: UniformType::_4f,
    },
    UniformName {
        name: std::borrow::Cow::Borrowed("u_radius"),
        type_: UniformType::_1f,
    },
    UniformName {
        name: std::borrow::Cow::Borrowed("u_color"),
        type_: UniformType::_4f,
    },
    UniformName {
        name: std::borrow::Cow::Borrowed("u_corner_radius"),
        type_: UniformType::_1f,
    },
];

/// Compile the shadow shader program. Called once at startup alongside the background shader.
pub fn compile_shadow_shader(
    renderer: &mut GlesRenderer,
) -> Option<smithay::backend::renderer::gles::GlesPixelProgram> {
    match renderer.compile_custom_pixel_shader(SHADOW_SHADER_SRC, SHADOW_UNIFORMS) {
        Ok(shader) => Some(shader),
        Err(e) => {
            tracing::error!("Failed to compile shadow shader: {e}");
            None
        }
    }
}

fn shadow_uniforms(
    shadow_padding: i32,
    content_w: i32,
    content_h: i32,
    shadow_radius: f32,
    corner_radius: f32,
) -> Vec<Uniform<'static>> {
    use srwc::config::DecorationConfig;
    let sc = DecorationConfig::SHADOW_COLOR;
    vec![
        Uniform::new(
            "u_window_rect",
            (
                shadow_padding as f32,
                shadow_padding as f32,
                content_w as f32,
                content_h as f32,
            ),
        ),
        Uniform::new("u_radius", shadow_radius),
        Uniform::new(
            "u_color",
            (
                sc[0] as f32 / 255.0,
                sc[1] as f32 / 255.0,
                sc[2] as f32 / 255.0,
                sc[3] as f32 / 255.0,
            ),
        ),
        Uniform::new("u_corner_radius", corner_radius),
    ]
}

const CORNER_CLIP_SRC: &str = include_str!("shaders/corner_clip.glsl");

pub const CORNER_CLIP_UNIFORMS: &[UniformName<'static>] = &[
    UniformName {
        name: Cow::Borrowed("u_size"),
        type_: UniformType::_2f,
    },
    UniformName {
        name: Cow::Borrowed("u_geo"),
        type_: UniformType::_4f,
    },
    UniformName {
        name: Cow::Borrowed("u_radius"),
        type_: UniformType::_1f,
    },
    UniformName {
        name: Cow::Borrowed("u_clip_top"),
        type_: UniformType::_1f,
    },
    UniformName {
        name: Cow::Borrowed("u_clip_shadow"),
        type_: UniformType::_1f,
    },
];

pub fn compile_corner_clip_shader(renderer: &mut GlesRenderer) -> Option<GlesTexProgram> {
    match renderer.compile_custom_texture_shader(CORNER_CLIP_SRC, CORNER_CLIP_UNIFORMS) {
        Ok(shader) => Some(shader),
        Err(e) => {
            tracing::error!("Failed to compile corner clip shader: {e}");
            None
        }
    }
}

/// Wrapper element that applies a rounded-corner clipping shader to a window's root surface.
pub struct RoundedCornerElement {
    inner: WaylandSurfaceRenderElement<GlesRenderer>,
    shader: GlesTexProgram,
    uniforms: Vec<Uniform<'static>>,
    corner_radius: f64,
    clip_top: bool,
}

impl RoundedCornerElement {
    pub fn new(
        inner: WaylandSurfaceRenderElement<GlesRenderer>,
        shader: GlesTexProgram,
        uniforms: Vec<Uniform<'static>>,
        corner_radius: f64,
        clip_top: bool,
    ) -> Self {
        Self {
            inner,
            shader,
            uniforms,
            corner_radius,
            clip_top,
        }
    }
}

impl Element for RoundedCornerElement {
    fn id(&self) -> &smithay::backend::renderer::element::Id {
        self.inner.id()
    }
    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }
    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.inner.location(scale)
    }
    fn src(&self) -> Rectangle<f64, smithay::utils::Buffer> {
        self.inner.src()
    }
    fn transform(&self) -> Transform {
        self.inner.transform()
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }
    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let regions = self.inner.opaque_regions(scale);
        if regions.is_empty() || self.corner_radius <= 0.0 {
            return regions;
        }
        let geo = self.geometry(scale);
        // +1 to cover anti-aliased fringe from smoothstep
        let r = (self.corner_radius * scale.x).ceil() as i32 + 1;
        let (w, h) = (geo.size.w, geo.size.h);
        if w <= 2 * r || h <= 2 * r {
            return regions;
        }
        let mut corners = Vec::with_capacity(4);
        if self.clip_top {
            corners.push(Rectangle::new((0, 0).into(), (r, r).into()));
            corners.push(Rectangle::new((w - r, 0).into(), (r, r).into()));
        }
        corners.push(Rectangle::new((0, h - r).into(), (r, r).into()));
        corners.push(Rectangle::new((w - r, h - r).into(), (r, r).into()));
        let rects: Vec<_> = regions.into_iter().collect();
        Rectangle::subtract_rects_many_in_place(rects, corners)
            .into_iter()
            .collect()
    }
    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }
    fn kind(&self) -> Kind {
        self.inner.kind()
    }
}

impl RenderElement<GlesRenderer> for RoundedCornerElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, smithay::utils::Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        _cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), GlesError> {
        frame.override_default_tex_program(self.shader.clone(), self.uniforms.clone());
        let result = self
            .inner
            .draw(frame, src, dst, damage, opaque_regions, _cache);
        frame.clear_tex_program_override();
        result
    }

    fn underlying_storage(
        &self,
        renderer: &mut GlesRenderer,
    ) -> Option<smithay::backend::renderer::element::UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }
}

/// Build render elements for X11 override-redirect windows (menus, tooltips, splashes).
/// Same camera/zoom math as managed windows.
fn build_override_redirect_elements(
    state: &crate::state::Srwc,
    renderer: &mut GlesRenderer,
    output: &Output,
    camera: Point<f64, Logical>,
    zoom: f64,
) -> Vec<OutputRenderElements> {
    let output_scale = output.current_scale().fractional_scale();
    let scale = Scale::from(output_scale);
    let viewport_size = crate::state::output_logical_size(output);
    let visible_rect = canvas::visible_canvas_rect(camera.to_i32_round(), viewport_size, zoom);

    let mut elements = Vec::new();
    // Reverse: newest OR window = topmost
    for or_surface in state.xwayland.override_redirect.iter().rev() {
        let Some(wl_surface) = or_surface.wl_surface() else {
            continue;
        };
        let canvas_pos = state.or_canvas_position(or_surface);
        let or_size = or_surface.geometry().size;
        let or_rect = Rectangle::new(canvas_pos, or_size);
        if !visible_rect.overlaps(or_rect) {
            continue;
        }

        let render_loc: Point<f64, Logical> = Point::from((
            canvas_pos.x as f64 - camera.x,
            canvas_pos.y as f64 - camera.y,
        ));
        let physical_loc: Point<f64, Physical> = render_loc.to_physical_precise_round(scale);
        let elems: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
            smithay::backend::renderer::element::surface::render_elements_from_surface_tree(
                renderer,
                &wl_surface,
                physical_loc.to_i32_round(),
                scale,
                1.0,
                Kind::Unspecified,
            );
        elements.extend(elems.into_iter().map(|elem| {
            OutputRenderElements::Window(RescaleRenderElement::from_element(
                elem,
                Point::<i32, Physical>::from((0, 0)),
                zoom,
            ))
        }));
    }
    elements
}

/// Build render elements for canvas-positioned layer surfaces (zoomed like windows).
/// Mirrors the window pipeline: position relative to camera, then RescaleRenderElement for zoom.
pub fn build_canvas_layer_elements(
    state: &crate::state::Srwc,
    renderer: &mut GlesRenderer,
    output: &Output,
    camera: Point<f64, smithay::utils::Logical>,
    zoom: f64,
) -> Vec<OutputRenderElements> {
    let output_scale = output.current_scale().fractional_scale();
    let mut elements = Vec::new();

    for cl in &state.canvas_layers {
        let Some(pos) = cl.position else {
            continue;
        };
        // Camera-relative position (same as render_elements_for_region does for windows)
        let rel: Point<f64, Logical> =
            Point::from((pos.x as f64 - camera.x, pos.y as f64 - camera.y));
        let physical_loc = rel.to_physical_precise_round(output_scale);

        let surface_elements = cl
            .surface
            .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                renderer,
                physical_loc,
                smithay::utils::Scale::from(output_scale),
                1.0,
            );
        elements.extend(surface_elements.into_iter().map(|elem| {
            OutputRenderElements::Window(RescaleRenderElement::from_element(
                elem,
                Point::<i32, Physical>::from((0, 0)),
                zoom,
            ))
        }));
    }

    elements
}

/// Build render elements for all layer surfaces on the given layer.
/// Layer surfaces are screen-fixed (not zoomed), so they use raw WaylandSurfaceRenderElement.
///
/// When `blur_config` is `Some`, layer surfaces whose `namespace()` matches a window rule
/// with `blur = true` will produce `BlurRequestData` entries alongside their render elements.
fn build_layer_elements(
    output: &Output,
    renderer: &mut GlesRenderer,
    layer: WlrLayer,
    blur_config: Option<(&srwc::config::Config, bool, BlurLayer)>,
) -> (Vec<OutputRenderElements>, Vec<BlurRequestData>) {
    let map = layer_map_for_output(output);
    let output_scale = output.current_scale().fractional_scale();
    let mut elements = Vec::new();
    let mut blur_requests = Vec::new();

    for surface in map.layers_on(layer).rev() {
        let geo = map.layer_geometry(surface).unwrap_or_default();
        let loc = geo.loc.to_physical_precise_round(output_scale);

        let elem_start = elements.len();
        elements.extend(
            surface
                .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                    renderer,
                    loc,
                    smithay::utils::Scale::from(output_scale),
                    1.0,
                )
                .into_iter()
                .map(OutputRenderElements::Layer),
        );

        if let Some((config, blur_enabled, layer_tag)) = blur_config
            && blur_enabled
            && config
                .match_window_rule(surface.namespace(), "")
                .is_some_and(|r| r.blur)
        {
            let elem_count = elements.len() - elem_start;
            let screen_rect = geo.to_physical_precise_round(output_scale);
            blur_requests.push(BlurRequestData {
                surface_id: Resource::id(surface.wl_surface()),
                screen_rect,
                elem_start,
                elem_count,
                layer: layer_tag,
            });
        }
    }

    (elements, blur_requests)
}

/// Resolve which xcursor name to load for the current cursor status.
/// Build the cursor render element(s) for the current frame.
/// `camera` and `zoom` are from the output being rendered.
/// Returns `OutputRenderElements` — either xcursor memory buffers or client surface elements.
pub fn build_cursor_elements(
    state: &mut crate::state::Srwc,
    renderer: &mut GlesRenderer,
    camera: Point<f64, smithay::utils::Logical>,
    zoom: f64,
    scale: f64,
    alpha: f32,
) -> Vec<OutputRenderElements> {
    if alpha <= 0.0 {
        return vec![];
    }
    let pointer = state.pointer();
    let canvas_pos = pointer.current_location();
    let screen_pos = canvas_to_screen(CanvasPos(canvas_pos), camera, zoom).0;
    let physical_pos: Point<f64, Physical> = screen_pos.to_physical_precise_round(scale);

    // Separate the status check from mutable state access (Rust 2024 borrow rules)
    let status = state.cursor.cursor_status.clone();
    let mut elements = match status {
        CursorImageStatus::Hidden => vec![],
        CursorImageStatus::Surface(ref surface) => {
            if !surface.alive() {
                state.cursor.cursor_status = CursorImageStatus::default_named();
                return build_xcursor_elements(state, renderer, physical_pos, "default", alpha);
            }
            let hotspot = with_states(surface, |states| {
                states
                    .data_map
                    .get::<CursorImageSurfaceData>()
                    .map(|d| d.lock().unwrap().hotspot)
                    .unwrap_or_default()
            });
            let pos: Point<i32, Physical> = (
                (physical_pos.x - hotspot.x as f64) as i32,
                (physical_pos.y - hotspot.y as f64) as i32,
            )
                .into();
            let elems: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                smithay::backend::renderer::element::surface::render_elements_from_surface_tree(
                    renderer,
                    surface,
                    pos,
                    Scale::from(1.0),
                    alpha,
                    Kind::Cursor,
                );
            elems
                .into_iter()
                .map(|e| OutputRenderElements::CursorSurface(e.into()))
                .collect()
        }
        CursorImageStatus::Named(icon) => {
            build_xcursor_elements(state, renderer, physical_pos, icon.name(), alpha)
        }
    };

    // Render DnD icon if active
    if let Some(dnd_icon) = state.dnd_icon.as_ref()
        && dnd_icon.surface.alive()
    {
        let icon_pos = screen_pos + dnd_icon.offset.to_f64();
        let physical_icon_pos: Point<i32, Physical> = icon_pos.to_physical_precise_round(scale);

        let dnd_elems: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
            smithay::backend::renderer::element::surface::render_elements_from_surface_tree(
                renderer,
                &dnd_icon.surface,
                physical_icon_pos,
                Scale::from(1.0),
                alpha,
                Kind::Unspecified,
            );
        elements.extend(
            dnd_elems
                .into_iter()
                .map(|e| OutputRenderElements::CursorSurface(e.into())),
        );
    }

    elements
}

/// Build xcursor memory buffer elements for a named cursor icon.
fn build_xcursor_elements(
    state: &mut crate::state::Srwc,
    renderer: &mut GlesRenderer,
    physical_pos: Point<f64, Physical>,
    name: &'static str,
    alpha: f32,
) -> Vec<OutputRenderElements> {
    let theme = state.config.cursor_theme.clone();
    let theme_deref = theme.as_deref();
    let loaded = state.load_xcursor(name, theme_deref).is_some();
    if !loaded && state.load_xcursor("default", theme_deref).is_none() {
        return vec![];
    }
    let key = if loaded { name } else { "default" };
    let cursor_frames = state.cursor.cursor_buffers.get(key).unwrap();

    // Select the active frame
    let frame_idx = if cursor_frames.total_duration_ms == 0 {
        0
    } else {
        let elapsed =
            state.start_time.elapsed().as_millis() as u32 % cursor_frames.total_duration_ms;
        let mut acc = 0u32;
        let mut idx = 0;
        for (i, &(_, _, delay)) in cursor_frames.frames.iter().enumerate() {
            acc += delay;
            if elapsed < acc {
                idx = i;
                break;
            }
        }
        idx
    };

    let (buffer, hotspot, _) = &cursor_frames.frames[frame_idx];
    let hotspot = *hotspot;

    let pos = physical_pos - Point::from((hotspot.x as f64, hotspot.y as f64));
    match MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        pos,
        buffer,
        Some(alpha),
        None,
        None,
        Kind::Cursor,
    ) {
        Ok(elem) => vec![OutputRenderElements::Cursor(elem)],
        Err(_) => vec![],
    }
}

/// Build render elements for a locked session: only the lock surface.
/// No compositor cursor — the lock client manages its own visuals.
fn compose_lock_frame(
    state: &crate::state::Srwc,
    renderer: &mut GlesRenderer,
    output: &Output,
    _cursor_elements: Vec<OutputRenderElements>,
) -> Vec<OutputRenderElements> {
    let mut elements = Vec::new();

    if let Some(lock_surface) = state.lock_surfaces.get(output) {
        let output_scale = output.current_scale().fractional_scale();
        let lock_elements =
            smithay::backend::renderer::element::surface::render_elements_from_surface_tree(
                renderer,
                lock_surface.wl_surface(),
                (0, 0),
                Scale::from(output_scale),
                1.0,
                Kind::Unspecified,
            );
        elements.extend(lock_elements.into_iter().map(OutputRenderElements::Layer));
    }

    elements
}

/// Assemble all render elements for a frame.
/// Caller provides cursor elements (built before taking the renderer).
pub fn compose_frame(
    state: &mut crate::state::Srwc,
    renderer: &mut GlesRenderer,
    output: &Output,
    cursor_elements: Vec<OutputRenderElements>,
) -> Vec<OutputRenderElements> {
    // Session lock: render only lock surface (or black) + cursor
    if !matches!(state.session_lock, crate::state::SessionLock::Unlocked) {
        return compose_lock_frame(state, renderer, output, cursor_elements);
    }

    // Screenshot UI: if open, render the frozen screenshot and overlay rects
    if state.screenshot.ui.is_open() {
        let mut all = cursor_elements;
        all.extend(state.screenshot.ui.render_output(renderer, output));
        return all;
    }

    // Ensure this output has a background element (lazy init per output, and re-init after config reload)
    if !state.render.cached_bg_elements.contains_key(&output.name())
        && !state.render.cached_wallpaper.contains_key(&output.name())
    {
        let output_size = crate::state::output_logical_size(output);
        init_background(state, renderer, output_size, &output.name());
    }

    // Read per-output state directly — not via active_output() which follows the pointer
    let (camera, zoom) = {
        let os = crate::state::output_state(output);
        (os.camera, os.zoom)
    };

    let viewport_size = crate::state::output_logical_size(output);
    let visible_rect = canvas::visible_canvas_rect(camera.to_i32_round(), viewport_size, zoom);
    let output_scale = output.current_scale().fractional_scale();
    let scale = Scale::from(output_scale);

    // Split windows into normal and widget layers so canvas layers render between them.
    // Replicates render_elements_for_region internals: bbox overlap, camera offset, zoom.
    let mut zoomed_normal: Vec<OutputRenderElements> = Vec::new();
    let mut zoomed_widgets: Vec<OutputRenderElements> = Vec::new();

    let blur_enabled = state.render.blur_down_shader.is_some()
        && state.render.blur_up_shader.is_some()
        && state.render.blur_mask_shader.is_some();
    let mut blur_requests: Vec<BlurRequestData> = Vec::new();

    // Focused surface for decoration focus state
    let focused_surface = state
        .seat
        .get_keyboard()
        .and_then(|kb| kb.current_focus())
        .map(|f| f.0);

    for window in state.space.elements().rev() {
        let Some(loc) = state.space.element_location(window) else {
            continue;
        };
        let geom_loc = window.geometry().loc;
        let geom_size = window.geometry().size;
        let Some(wl_surface) = window.wl_surface() else {
            continue;
        };
        let is_fullscreen = state.fullscreen.values().any(|fs| &fs.window == window);
        let has_ssd = !is_fullscreen && state.decorations.contains_key(&Resource::id(&*wl_surface));

        let mut bbox = window.bbox();
        bbox.loc += loc - geom_loc;
        if has_ssd {
            let r = srwc::config::DecorationConfig::SHADOW_RADIUS.ceil() as i32;
            let bar = srwc::config::DecorationConfig::TITLE_BAR_HEIGHT;
            bbox.loc.x -= r;
            bbox.loc.y -= bar + r;
            bbox.size.w += 2 * r;
            bbox.size.h += bar + 2 * r;
        }
        if !visible_rect.overlaps(bbox) {
            continue;
        }

        let render_loc: Point<f64, Logical> = Point::from((
            loc.x as f64 - geom_loc.x as f64 - camera.x,
            loc.y as f64 - geom_loc.y as f64 - camera.y,
        ));
        let applied = srwc::config::applied_rule(&wl_surface);
        let is_widget = applied.as_ref().is_some_and(|r| r.widget);
        let wants_blur = blur_enabled && applied.as_ref().is_some_and(|r| r.blur);
        let opacity = applied.as_ref().and_then(|r| r.opacity).unwrap_or(1.0);

        let elems = window.render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
            renderer,
            render_loc.to_physical_precise_round(scale),
            scale,
            opacity as f32,
        );

        let target = if is_widget {
            &mut zoomed_widgets
        } else {
            &mut zoomed_normal
        };
        let elem_start = target.len();
        let mut shadow_count = 0usize;

        if has_ssd {
            let bar_height = srwc::config::DecorationConfig::TITLE_BAR_HEIGHT;
            let is_focused = focused_surface.as_ref().is_some_and(|f| *f == *wl_surface);

            // Update decoration state (re-render title bar if needed)
            if let Some(deco) = state.decorations.get_mut(&wl_surface.id()) {
                deco.update(geom_size.w, is_focused, &state.config.decorations);
            }

            // Title bar element: positioned above the window
            if let Some(deco) = state.decorations.get(&wl_surface.id()) {
                let bar_loc: Point<f64, Logical> =
                    Point::from((render_loc.x, render_loc.y - bar_height as f64));
                let bar_physical: Point<f64, Physical> = bar_loc.to_physical_precise_round(scale);
                let bar_alpha = if opacity < 1.0 {
                    Some(opacity as f32)
                } else {
                    None
                };
                if let Ok(bar_elem) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    bar_physical,
                    &deco.title_bar,
                    bar_alpha,
                    None,
                    None,
                    Kind::Unspecified,
                ) {
                    target.push(OutputRenderElements::Decoration(
                        RescaleRenderElement::from_element(
                            bar_elem,
                            Point::<i32, Physical>::from((0, 0)),
                            zoom,
                        ),
                    ));
                }
            }

            // Window surface elements — clip bottom corners to match title bar rounding
            if let Some(ref shader) = state.render.corner_clip_shader {
                let radius = state.config.decorations.corner_radius as f32;
                if radius > 0.0 {
                    let toplevel_id =
                        smithay::backend::renderer::element::Id::from_wayland_resource(
                            &*wl_surface,
                        );
                    for elem in elems {
                        if *elem.id() == toplevel_id {
                            let buf = elem.buffer_size();
                            // SSD windows: geometry is the full buffer, only clip bottom corners
                            let uniforms = vec![
                                Uniform::new("u_size", (buf.w as f32, buf.h as f32)),
                                Uniform::new("u_geo", (0.0f32, 0.0f32, buf.w as f32, buf.h as f32)),
                                Uniform::new("u_radius", radius),
                                Uniform::new("u_clip_top", 0.0f32),
                                Uniform::new("u_clip_shadow", 0.0f32),
                            ];
                            target.push(OutputRenderElements::CsdWindow(
                                RescaleRenderElement::from_element(
                                    RoundedCornerElement::new(
                                        elem,
                                        shader.clone(),
                                        uniforms,
                                        radius as f64,
                                        false,
                                    ),
                                    Point::<i32, Physical>::from((0, 0)),
                                    zoom,
                                ),
                            ));
                        } else {
                            target.push(OutputRenderElements::Window(
                                RescaleRenderElement::from_element(
                                    elem,
                                    Point::<i32, Physical>::from((0, 0)),
                                    zoom,
                                ),
                            ));
                        }
                    }
                } else {
                    target.extend(elems.into_iter().map(|elem| {
                        OutputRenderElements::Window(RescaleRenderElement::from_element(
                            elem,
                            Point::<i32, Physical>::from((0, 0)),
                            zoom,
                        ))
                    }));
                }
            } else {
                target.extend(elems.into_iter().map(|elem| {
                    OutputRenderElements::Window(RescaleRenderElement::from_element(
                        elem,
                        Point::<i32, Physical>::from((0, 0)),
                        zoom,
                    ))
                }));
            }

            // Shadow element: cached per-window, rebuilt only on resize.
            // Stable Id lets the damage tracker skip unchanged shadow regions.
            if let Some(ref shader) = state.render.shadow_shader {
                use srwc::config::DecorationConfig;
                let radius = DecorationConfig::SHADOW_RADIUS;
                let r = radius.ceil() as i32;
                let shadow_w = geom_size.w + 2 * r;
                let shadow_h = geom_size.h + bar_height + 2 * r;
                let shadow_loc: Point<i32, Logical> = Point::from((
                    render_loc.x.round() as i32 - r,
                    render_loc.y.round() as i32 - bar_height - r,
                ));
                let shadow_area = Rectangle::new(shadow_loc, (shadow_w, shadow_h).into());
                let corner_r = state.config.decorations.corner_radius as f32;

                if let Some(deco) = state.decorations.get_mut(&wl_surface.id()) {
                    let content_size = (geom_size.w, geom_size.h);
                    if deco
                        .cached_shadow
                        .as_ref()
                        .is_some_and(|s| (s.alpha() - opacity as f32).abs() > f32::EPSILON)
                    {
                        deco.cached_shadow = None;
                    }
                    let shadow_elem = if let Some(shadow) = &mut deco.cached_shadow {
                        if deco.shadow_content_size != content_size {
                            deco.shadow_content_size = content_size;
                            shadow.update_uniforms(shadow_uniforms(
                                r,
                                geom_size.w,
                                geom_size.h + bar_height,
                                radius,
                                corner_r,
                            ));
                        }
                        shadow.resize(shadow_area, None);
                        shadow.clone()
                    } else {
                        deco.shadow_content_size = content_size;
                        let elem = PixelShaderElement::new(
                            shader.clone(),
                            shadow_area,
                            None,
                            opacity as f32,
                            shadow_uniforms(
                                r,
                                geom_size.w,
                                geom_size.h + bar_height,
                                radius,
                                corner_r,
                            ),
                            Kind::Unspecified,
                        );
                        deco.cached_shadow = Some(elem.clone());
                        elem
                    };
                    target.push(OutputRenderElements::Background(
                        RescaleRenderElement::from_element(
                            shadow_elem,
                            Point::<i32, Physical>::from((0, 0)),
                            zoom,
                        ),
                    ));
                    shadow_count = 1;
                }
            }
        } else if let Some(ref shader) = state.render.corner_clip_shader {
            let geo = window.geometry();
            let radius = state.config.decorations.corner_radius as f32;

            let rule_forced = applied
                .as_ref()
                .is_some_and(|r| r.decoration != srwc::config::DecorationMode::Client);

            if !rule_forced && !is_fullscreen {
                if radius > 0.0 {
                    let toplevel_id =
                        smithay::backend::renderer::element::Id::from_wayland_resource(
                            &*wl_surface,
                        );
                    for elem in elems {
                        if *elem.id() == toplevel_id {
                            let buf = elem.buffer_size();
                            let uniforms = vec![
                                Uniform::new("u_size", (buf.w as f32, buf.h as f32)),
                                Uniform::new(
                                    "u_geo",
                                    (
                                        geo.loc.x as f32,
                                        geo.loc.y as f32,
                                        geo.size.w as f32,
                                        geo.size.h as f32,
                                    ),
                                ),
                                Uniform::new("u_radius", radius),
                                Uniform::new("u_clip_top", 1.0f32),
                                Uniform::new("u_clip_shadow", 1.0f32),
                            ];
                            target.push(OutputRenderElements::CsdWindow(
                                RescaleRenderElement::from_element(
                                    RoundedCornerElement::new(
                                        elem,
                                        shader.clone(),
                                        uniforms,
                                        radius as f64,
                                        true,
                                    ),
                                    Point::<i32, Physical>::from((0, 0)),
                                    zoom,
                                ),
                            ));
                        } else {
                            target.push(OutputRenderElements::Window(
                                RescaleRenderElement::from_element(
                                    elem,
                                    Point::<i32, Physical>::from((0, 0)),
                                    zoom,
                                ),
                            ));
                        }
                    }
                } else {
                    target.extend(elems.into_iter().map(|elem| {
                        OutputRenderElements::Window(RescaleRenderElement::from_element(
                            elem,
                            Point::<i32, Physical>::from((0, 0)),
                            zoom,
                        ))
                    }));
                }

                // Compositor shadow behind CSD windows
                if let Some(ref shadow_shader) = state.render.shadow_shader {
                    use srwc::config::DecorationConfig;
                    let shadow_radius = DecorationConfig::SHADOW_RADIUS;
                    let sr = shadow_radius.ceil() as i32;
                    let shadow_w = geom_size.w + 2 * sr;
                    let shadow_h = geom_size.h + 2 * sr;
                    // render_loc is the buffer origin; geometry starts at render_loc + geo.loc
                    let shadow_loc: Point<i32, Logical> = Point::from((
                        render_loc.x.round() as i32 + geo.loc.x - sr,
                        render_loc.y.round() as i32 + geo.loc.y - sr,
                    ));
                    let shadow_area = Rectangle::new(shadow_loc, (shadow_w, shadow_h).into());
                    let content_size = (geom_size.w, geom_size.h);
                    let corner_r = state.config.decorations.corner_radius as f32;

                    let shadow_entry = state.render.csd_shadows.entry(wl_surface.id());
                    let (shadow_elem, cached_size) = shadow_entry.or_insert_with(|| {
                        let elem = PixelShaderElement::new(
                            shadow_shader.clone(),
                            shadow_area,
                            None,
                            opacity as f32,
                            shadow_uniforms(sr, geom_size.w, geom_size.h, shadow_radius, corner_r),
                            Kind::Unspecified,
                        );
                        (elem, content_size)
                    });

                    if *cached_size != content_size {
                        *cached_size = content_size;
                        shadow_elem.update_uniforms(shadow_uniforms(
                            sr,
                            geom_size.w,
                            geom_size.h,
                            shadow_radius,
                            corner_r,
                        ));
                    }
                    shadow_elem.resize(shadow_area, None);
                    target.push(OutputRenderElements::Background(
                        RescaleRenderElement::from_element(
                            shadow_elem.clone(),
                            Point::<i32, Physical>::from((0, 0)),
                            zoom,
                        ),
                    ));
                    shadow_count = 1;
                }
            } else {
                target.extend(elems.into_iter().map(|elem| {
                    OutputRenderElements::Window(RescaleRenderElement::from_element(
                        elem,
                        Point::<i32, Physical>::from((0, 0)),
                        zoom,
                    ))
                }));
            }
        } else {
            target.extend(elems.into_iter().map(|elem| {
                OutputRenderElements::Window(RescaleRenderElement::from_element(
                    elem,
                    Point::<i32, Physical>::from((0, 0)),
                    zoom,
                ))
            }));
        }

        if wants_blur {
            let elem_count = target.len() - elem_start - shadow_count;
            let screen_loc: Point<i32, Logical> =
                Point::from(((render_loc.x * zoom) as i32, (render_loc.y * zoom) as i32));
            let screen_size: Size<i32, Logical> = if has_ssd {
                let bar = srwc::config::DecorationConfig::TITLE_BAR_HEIGHT;
                (
                    (geom_size.w as f64 * zoom).ceil() as i32,
                    ((geom_size.h + bar) as f64 * zoom).ceil() as i32,
                )
                    .into()
            } else {
                (
                    (geom_size.w as f64 * zoom).ceil() as i32,
                    (geom_size.h as f64 * zoom).ceil() as i32,
                )
                    .into()
            };
            let screen_rect = Rectangle::new(
                if has_ssd {
                    Point::from((
                        screen_loc.x,
                        screen_loc.y
                            - (srwc::config::DecorationConfig::TITLE_BAR_HEIGHT as f64 * zoom)
                                as i32,
                    ))
                } else {
                    // CSD windows: geometry starts at render_loc + geo.loc, not at render_loc
                    let geo = window.geometry();
                    Point::from((
                        ((render_loc.x + geo.loc.x as f64) * zoom) as i32,
                        ((render_loc.y + geo.loc.y as f64) * zoom) as i32,
                    ))
                },
                screen_size,
            )
            .to_physical_precise_round(output_scale);
            blur_requests.push(BlurRequestData {
                surface_id: wl_surface.id(),
                screen_rect,
                elem_start,
                elem_count,
                layer: if is_widget {
                    BlurLayer::Widget
                } else {
                    BlurLayer::Normal
                },
            });
        }
    }

    let canvas_layer_elements = build_canvas_layer_elements(state, renderer, output, camera, zoom);

    let or_elements = build_override_redirect_elements(state, renderer, output, camera, zoom);

    let outline_elements =
        build_output_outline_elements(state, renderer, output, camera, zoom, viewport_size);

    let bg_elements: Vec<OutputRenderElements> =
        // Wallpaper: static image, no zoom applied (rendered fullscreen fixed)
        if let Some((tex, id)) = state.render.cached_wallpaper.get(&output.name()) {
            let output_size = crate::state::output_logical_size(output);
            use smithay::backend::renderer::Renderer;
            let elem = TextureRenderElement::from_static_texture(
                id.clone(),
                renderer.context_id(),
                Point::from((0.0f64, 0.0f64)),
                tex.clone(),
                1,
                Transform::Normal,
                None,
                None,
                Some(Size::from((output_size.w, output_size.h))),
                None,
                Kind::Unspecified,
            );
            vec![OutputRenderElements::Blur(elem)]
        } else if let Some(elem) = state.render.cached_bg_elements.get(&output.name()) {
            vec![OutputRenderElements::Background(
                RescaleRenderElement::from_element(
                    elem.clone(),
                    Point::<i32, Physical>::from((0, 0)),
                    zoom,
                ),
            )]
        } else {
            vec![]
        };

    let is_fullscreen = state.is_output_fullscreen(output);
    let (overlay_elements, overlay_blur) = build_layer_elements(
        output,
        renderer,
        WlrLayer::Overlay,
        Some((&state.config, blur_enabled, BlurLayer::Overlay)),
    );
    let (top_elements, top_blur) = if !is_fullscreen {
        build_layer_elements(
            output,
            renderer,
            WlrLayer::Top,
            Some((&state.config, blur_enabled, BlurLayer::Top)),
        )
    } else {
        (vec![], vec![])
    };
    let (bottom_elements, _) = if !is_fullscreen {
        build_layer_elements(output, renderer, WlrLayer::Bottom, None)
    } else {
        (vec![], vec![])
    };
    let (background_layer_elements, _) =
        build_layer_elements(output, renderer, WlrLayer::Background, None);

    // Compute prefix offsets so we know where each group lands in all_elements
    let overlay_prefix = cursor_elements.len() + or_elements.len();
    let top_prefix = overlay_prefix + overlay_elements.len();
    let normal_prefix = top_prefix + top_elements.len();
    let widget_prefix = normal_prefix + zoomed_normal.len() + canvas_layer_elements.len();

    // Merge blur requests: layer surfaces first (front-to-back), then windows
    let mut all_blur_requests: Vec<BlurRequestData> = Vec::new();
    all_blur_requests.extend(overlay_blur);
    all_blur_requests.extend(top_blur);
    all_blur_requests.extend(blur_requests);

    let mut all_elements: Vec<OutputRenderElements> = Vec::with_capacity(
        cursor_elements.len()
            + or_elements.len()
            + overlay_elements.len()
            + top_elements.len()
            + zoomed_normal.len()
            + canvas_layer_elements.len()
            + zoomed_widgets.len()
            + bottom_elements.len()
            + outline_elements.len()
            + bg_elements.len()
            + background_layer_elements.len(),
    );
    all_elements.extend(cursor_elements);
    all_elements.extend(or_elements);
    all_elements.extend(overlay_elements);
    all_elements.extend(top_elements);
    all_elements.extend(zoomed_normal);
    all_elements.extend(canvas_layer_elements);
    all_elements.extend(zoomed_widgets);
    all_elements.extend(bottom_elements);
    all_elements.extend(outline_elements);
    all_elements.extend(bg_elements);
    all_elements.extend(background_layer_elements);

    // Process blur requests: render behind-content, blur, insert
    if !all_blur_requests.is_empty() {
        process_blur_requests(
            state,
            renderer,
            output,
            output_scale,
            &mut all_elements,
            &all_blur_requests,
            overlay_prefix,
            top_prefix,
            normal_prefix,
            widget_prefix,
        );
    }

    // Prune stale blur cache entries
    if blur_enabled {
        let active_ids: std::collections::HashSet<_> = all_blur_requests
            .iter()
            .map(|r| r.surface_id.clone())
            .collect();
        state
            .render
            .blur_cache
            .retain(|id, _| active_ids.contains(id));
    }

    all_elements
}

/// Draw thin outlines showing where other monitors' viewports sit on the canvas.
fn build_output_outline_elements(
    state: &crate::state::Srwc,
    renderer: &mut GlesRenderer,
    output: &Output,
    camera: Point<f64, Logical>,
    zoom: f64,
    viewport_size: Size<i32, Logical>,
) -> Vec<OutputRenderElements> {
    let thickness = state.config.output_outline.thickness;
    if thickness <= 0 {
        return vec![];
    }

    let opacity = state.config.output_outline.opacity as f32;
    if opacity <= 0.0 {
        return vec![];
    }
    let color = state.config.output_outline.color;
    let scale = output.current_scale().fractional_scale();

    let mut elements = Vec::new();

    for other in state.space.outputs() {
        if *other == *output {
            continue;
        }

        let (other_camera, other_zoom) = {
            let os = crate::state::output_state(other);
            (os.camera, os.zoom)
        };
        let other_size = crate::state::output_logical_size(other);

        // Other output's visible canvas rect
        let other_canvas =
            canvas::visible_canvas_rect(other_camera.to_i32_round(), other_size, other_zoom);

        // Transform to screen coords on *this* output
        let screen_x = ((other_canvas.loc.x as f64 - camera.x) * zoom) as i32;
        let screen_y = ((other_canvas.loc.y as f64 - camera.y) * zoom) as i32;
        let screen_w = (other_canvas.size.w as f64 * zoom) as i32;
        let screen_h = (other_canvas.size.h as f64 * zoom) as i32;

        // Clip to viewport
        let vp = Rectangle::from_size(viewport_size);
        let outline_rect = Rectangle::new((screen_x, screen_y).into(), (screen_w, screen_h).into());
        if !vp.overlaps(outline_rect) {
            continue;
        }

        // Draw 4 edges as thin filled buffers
        let edges: [(i32, i32, i32, i32); 4] = [
            (screen_x, screen_y, screen_w, thickness), // top
            (
                screen_x,
                screen_y + screen_h - thickness,
                screen_w,
                thickness,
            ), // bottom
            (screen_x, screen_y, thickness, screen_h), // left
            (
                screen_x + screen_w - thickness,
                screen_y,
                thickness,
                screen_h,
            ), // right
        ];

        for (ex, ey, ew, eh) in edges {
            // Clip edge to viewport
            let x0 = ex.max(0);
            let y0 = ey.max(0);
            let x1 = (ex + ew).min(viewport_size.w);
            let y1 = (ey + eh).min(viewport_size.h);
            if x1 <= x0 || y1 <= y0 {
                continue;
            }

            let w = x1 - x0;
            let h = y1 - y0;

            let pixels: Vec<u8> = vec![color[0], color[1], color[2], color[3]]
                .into_iter()
                .cycle()
                .take((w * h) as usize * 4)
                .collect();

            let buf = MemoryRenderBuffer::from_slice(
                &pixels,
                Fourcc::Abgr8888,
                (w, h),
                1,
                Transform::Normal,
                None,
            );

            let loc: Point<f64, Physical> = Point::from((x0, y0)).to_f64().to_physical(scale);
            if let Ok(elem) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                loc,
                &buf,
                Some(opacity),
                None,
                None,
                Kind::Unspecified,
            ) {
                elements.push(OutputRenderElements::Decoration(
                    RescaleRenderElement::from_element(
                        elem,
                        Point::<i32, Physical>::from((0, 0)),
                        1.0,
                    ),
                ));
            }
        }
    }

    elements
}

/// Sync foreign-toplevel protocol state with the current window list.
/// Call once per frame iteration (not per-output).
pub fn refresh_foreign_toplevels(state: &mut crate::state::Srwc) {
    let keyboard = state.keyboard();
    let focused = keyboard.current_focus().map(|f| f.0);
    let outputs: Vec<Output> = state.space.outputs().cloned().collect();
    srwc::protocols::foreign_toplevel::refresh::<crate::state::Srwc>(
        &mut state.foreign_toplevel_state,
        &state.space,
        focused.as_ref(),
        &outputs,
    );
}

/// Post-render: frame callbacks, space cleanup.
pub fn post_render(state: &mut crate::state::Srwc, output: &Output) {
    let time = state.start_time.elapsed();

    // Only send frame callbacks to visible windows — off-screen clients
    // naturally throttle to zero FPS without callbacks.
    let (camera, zoom) = {
        let os = crate::state::output_state(output);
        (os.camera, os.zoom)
    };
    let viewport_size = crate::state::output_logical_size(output);
    let visible_rect = canvas::visible_canvas_rect(camera.to_i32_round(), viewport_size, zoom);

    for window in state.space.elements() {
        let Some(loc) = state.space.element_location(window) else {
            continue;
        };
        let geom_loc = window.geometry().loc;
        let mut bbox = window.bbox();
        bbox.loc += loc - geom_loc;
        if !visible_rect.overlaps(bbox) {
            // For off-screen windows being screencast, only send the frame callback
            // from the focused output's post_render pass — avoids N callbacks on
            // N-output setups.
            let is_focused = state.focused_output.as_ref().is_some_and(|fo| fo == output);
            let is_being_cast = is_focused
                && state.screencast.screencasting.as_ref().is_some_and(|sc| {
                    window.wl_surface().is_some_and(|s| {
                        let wl_id = u64::from(s.id().protocol_id());
                        sc.casts
                            .iter()
                            .any(|c| c.is_active() && c.target == CastTarget::Window { id: wl_id })
                    })
                });
            if !is_being_cast {
                continue;
            }
        }
        window.send_frame(output, time, Some(Duration::ZERO), |_, _| {
            Some(output.clone())
        });
    }

    // Layer surface frame callbacks
    {
        let layer_map = layer_map_for_output(output);
        for layer_surface in layer_map.layers() {
            layer_surface.send_frame(output, time, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
    }

    // Canvas-positioned layer surface frame callbacks
    for cl in &state.canvas_layers {
        cl.surface
            .send_frame(output, time, Some(Duration::ZERO), |_, _| {
                Some(output.clone())
            });
    }

    // Override-redirect X11 surface frame callbacks
    for or_surface in &state.xwayland.override_redirect {
        if let Some(wl_surface) = or_surface.wl_surface() {
            smithay::desktop::utils::send_frames_surface_tree(
                &wl_surface,
                output,
                time,
                Some(Duration::ZERO),
                |_, _| Some(output.clone()),
            );
        }
    }

    // Cursor surface frame callbacks (animated cursors need these to advance)
    if let CursorImageStatus::Surface(ref surface) = state.cursor.cursor_status {
        smithay::desktop::utils::send_frames_surface_tree(
            surface,
            output,
            time,
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        );
    }

    // DnD icon surface frame callbacks
    if let Some(ref dnd_icon) = state.dnd_icon
        && dnd_icon.surface.alive()
    {
        smithay::desktop::utils::send_frames_surface_tree(
            &dnd_icon.surface,
            output,
            time,
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        );
    }

    // Lock surface frame callback
    if let Some(lock_surface) = state.lock_surfaces.get(output) {
        smithay::desktop::utils::send_frames_surface_tree(
            lock_surface.wl_surface(),
            output,
            time,
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        );
    }

    // Cleanup
    state.space.refresh();
    state.popups.cleanup();
    layer_map_for_output(output).cleanup();
}
