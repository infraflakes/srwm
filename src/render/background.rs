use smithay::{
    backend::renderer::{
        element::{Id, Kind},
        gles::{GlesRenderer, Uniform, element::PixelShaderElement},
    },
    output::Output,
    utils::{Point, Rectangle, Size},
};

use smithay::backend::allocator::Fourcc;

use super::BG_UNIFORMS;

/// Update the cached background shader element for the current camera/zoom.
/// Returns (camera_moved, zoom_changed) for the caller's damage logic.
pub fn update_background_element(
    state: &mut crate::state::Srwc,
    output: &Output,
    cur_camera: Point<f64, smithay::utils::Logical>,
    cur_zoom: f64,
    last_rendered_camera: Point<f64, smithay::utils::Logical>,
    last_rendered_zoom: f64,
) -> (bool, bool) {
    let camera_moved = cur_camera != last_rendered_camera;
    let zoom_changed = cur_zoom != last_rendered_zoom;
    let output_name = output.name();
    let output_size = crate::state::output_logical_size(output);
    let canvas_w = (output_size.w as f64 / cur_zoom).ceil() as i32;
    let canvas_h = (output_size.h as f64 / cur_zoom).ceil() as i32;
    let canvas_area = Rectangle::from_size((canvas_w, canvas_h).into());

    if let Some(elem) = state.render.cached_bg_elements.get_mut(&output_name) {
        elem.resize(canvas_area, Some(vec![canvas_area]));
        elem.update_uniforms(vec![Uniform::new(
            "u_camera",
            (cur_camera.x as f32, cur_camera.y as f32),
        )]);
    }
    (camera_moved, zoom_changed)
}

/// Compile background shader and/or load wallpaper image.
/// Called at startup and on config reload (lazy re-init).
/// On failure, falls back to `DEFAULT_SHADER` — never leaves background uninitialized.
pub fn init_background(
    state: &mut crate::state::Srwc,
    renderer: &mut GlesRenderer,
    initial_size: Size<i32, smithay::utils::Logical>,
    output_name: &str,
) {
    // Static wallpaper: load image as plain GlesTexture (no shader needed)
    // Priority: shader_path > wallpaper_path > default
    if state.config.background.shader_path.is_none()
        && let Some(path) = state.config.background.wallpaper_path.as_deref()
    {
        match image::open(path) {
            Ok(img) => {
                let img = img.into_rgba8();
                let (w, h) = img.dimensions();
                let raw = img.into_raw();

                use smithay::backend::renderer::ImportMem;
                use smithay::utils::Buffer;
                match renderer.import_memory(
                    &raw,
                    Fourcc::Abgr8888,
                    Size::<i32, Buffer>::from((w as i32, h as i32)),
                    false,
                ) {
                    Ok(texture) => {
                        state
                            .render
                            .cached_wallpaper
                            .insert(output_name.to_string(), (texture, Id::new()));
                        return;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to upload wallpaper texture: {e}, falling back to shader"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to load wallpaper image {path}: {e}, falling back to shader"
                );
            }
        }
    }

    // Reuse cached shader if already compiled (avoids redundant GPU work
    // when multiple outputs each need a background element).
    let shader = if let Some(ref cached) = state.render.background_shader {
        cached.clone()
    } else {
        let shader_source = if let Some(path) = state.config.background.shader_path.as_deref() {
            match std::fs::read_to_string(path) {
                Ok(src) => src,
                Err(e) => {
                    tracing::error!("Failed to read shader {path}: {e}, using default");
                    srwc::config::DEFAULT_SHADER.to_string()
                }
            }
        } else {
            srwc::config::DEFAULT_SHADER.to_string()
        };

        let compiled = match renderer.compile_custom_pixel_shader(&shader_source, BG_UNIFORMS) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to compile shader: {e}, using default");
                renderer
                    .compile_custom_pixel_shader(srwc::config::DEFAULT_SHADER, BG_UNIFORMS)
                    .expect("Default shader must compile")
            }
        };
        state.render.background_shader = Some(compiled.clone());
        compiled
    };

    let area = Rectangle::from_size(initial_size);
    state.render.cached_bg_elements.insert(
        output_name.to_string(),
        PixelShaderElement::new(
            shader,
            area,
            Some(vec![area]),
            1.0,
            vec![Uniform::new("u_camera", (0.0f32, 0.0f32))],
            Kind::Unspecified,
        ),
    );
}
