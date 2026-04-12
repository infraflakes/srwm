use smithay::{
    backend::{
        renderer::{ImportDma, damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::{
        EventLoop,
        timer::{TimeoutAction, Timer},
    },
    utils::{Point, Transform},
};
use std::time::Duration;

use crate::backend::Backend;
use crate::render::build_cursor_elements;
use crate::state::{Srwc, init_output_state, output_logical_size};

/// Initialize the winit backend: create a window, set up the output, and
/// start the render loop timer.
pub fn init_winit(
    event_loop: &mut EventLoop<'static, Srwc>,
    data: &mut Srwc,
) -> Result<(), Box<dyn std::error::Error>> {
    let (backend, mut winit_evt) = winit::init::<GlesRenderer>()?;
    let size = backend.window_size();

    // Store backend on state so protocol handlers can access the renderer
    data.backend = Some(Backend::Winit(Box::new(backend)));
    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(), // unknown physical size
            subpixel: Subpixel::Unknown,
            make: "srwc".to_string(),
            model: "winit".to_string(),
            serial_number: String::new(),
        },
    );
    let mode = Mode {
        size,
        refresh: 60_000, // 60 Hz in mHz
    };
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, None);
    output.set_preferred(mode);

    // Advertise the output as a wl_output global so clients can see it
    output.create_global::<crate::state::Srwc>(&data.display_handle);

    // Create DMA-BUF global — advertise GPU buffer formats to clients
    let formats = data.backend.as_mut().unwrap().renderer().dmabuf_formats();
    let dmabuf_global = data
        .dmabuf_state
        .create_global::<crate::state::Srwc>(&data.display_handle, formats);
    data.dmabuf_global = Some(dmabuf_global);

    {
        let mut backend = data.backend.take().unwrap();
        crate::render::init_background(data, backend.renderer(), size.to_logical(1), "winit");
        data.render.shadow_shader = crate::render::compile_shadow_shader(backend.renderer());
        data.render.corner_clip_shader =
            crate::render::compile_corner_clip_shader(backend.renderer());
        let (blur_down, blur_up, blur_mask) =
            crate::render::compile_blur_shaders(backend.renderer());
        data.render.blur_down_shader = blur_down;
        data.render.blur_up_shader = blur_up;
        data.render.blur_mask_shader = blur_mask;
        data.backend = Some(backend);
    }

    // Centre the viewport so canvas origin (0, 0) is in the middle of the screen
    let logical_size = size.to_logical(1);
    let initial_camera = Point::from((
        -(logical_size.w as f64) / 2.0,
        -(logical_size.h as f64) / 2.0,
    ));

    // Initialize per-output state for this output
    init_output_state(
        &output,
        initial_camera,
        data.config.nav.friction,
        Point::from((0, 0)),
    );

    // Restore saved camera/zoom from previous session
    let saved = crate::state::load_cameras();
    if let Some(&(saved_cam, saved_zoom)) = saved.get("winit") {
        let mut os = crate::state::output_state(&output);
        os.camera = saved_cam;
        os.zoom = saved_zoom;
    }
    data.focused_output = Some(output.clone());

    // Map the output into the space at the initial camera position
    data.space
        .map_output(&output, initial_camera.to_i32_round());

    // Notify output management clients about the winit output
    {
        use srwc::protocols::output_management::{ModeInfo, OutputHeadState};
        let mut heads = std::collections::HashMap::new();
        heads.insert(
            "winit".to_string(),
            OutputHeadState {
                name: "winit".to_string(),
                description: "srwc winit virtual output".to_string(),
                make: "srwc".to_string(),
                model: "winit".to_string(),
                serial_number: String::new(),
                physical_size: (0, 0),
                modes: vec![ModeInfo {
                    width: size.w,
                    height: size.h,
                    refresh: 60_000,
                    preferred: true,
                }],
                current_mode_index: Some(0),
                position: (0, 0),
                transform: Transform::Flipped180,
                scale: 1.0,
            },
        );
        srwc::protocols::output_management::notify_changes::<crate::state::Srwc>(
            &mut data.output_management_state,
            heads,
        );
    }

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    // Render loop: fires immediately, then re-arms at ~60fps
    let timer = Timer::immediate();
    event_loop
        .handle()
        .insert_source(timer, move |_, _, data| {
            // --- Dispatch winit events ---
            let mut stop = false;
            winit_evt.dispatch_new_events(|event| match event {
                WinitEvent::Resized { size, scale_factor } => {
                    let new_mode = Mode {
                        size,
                        refresh: 60_000,
                    };
                    output.change_current_state(
                        Some(new_mode),
                        None,
                        Some(smithay::output::Scale::Fractional(scale_factor)),
                        None,
                    );
                }
                WinitEvent::Input(event) => {
                    data.process_input_event(event);
                }
                WinitEvent::CloseRequested => {
                    stop = true;
                }
                _ => {}
            });

            if stop {
                data.loop_signal.stop();
                return TimeoutAction::Drop;
            }

            // --- Flush Wayland client messages before rendering ---
            data.display_handle.flush_clients().ok();

            // --- Delta time ---
            let now = std::time::Instant::now();
            let dt = {
                let mut os = crate::state::output_state(&output);
                let dt = (now - os.last_frame_instant).min(std::time::Duration::from_millis(33));
                os.last_frame_instant = now;
                dt
            };

            // --- Key repeat for compositor bindings ---
            data.apply_key_repeat();

            // --- Scroll momentum ---
            data.apply_scroll_momentum(dt);

            // --- Edge auto-pan (window drag near viewport edges) ---
            data.apply_edge_pan();

            // --- Zoom animation (before camera so recomputed target is used) ---
            data.apply_zoom_animation(dt);

            // --- Camera animation (window navigation) ---
            data.apply_camera_animation(dt);

            // --- Exec loading cursor timeout ---
            data.check_exec_cursor_timeout();

            // --- Read per-output state for this frame ---
            let (cur_camera, cur_zoom, last_cam, last_zoom) = {
                let os = crate::state::output_state(&output);
                (
                    os.camera,
                    os.zoom,
                    os.last_rendered_camera,
                    os.last_rendered_zoom,
                )
            };

            // --- Update cached background element ---
            let (camera_moved, zoom_changed) = crate::render::update_background_element(
                data, &output, cur_camera, cur_zoom, last_cam, last_zoom,
            );

            // --- Take backend to split borrow from state ---
            let Backend::Winit(mut backend) = data.backend.take().unwrap() else {
                unreachable!("winit timer with non-winit backend");
            };

            // --- Build cursor + compose frame ---
            let (cursor_cam, cursor_zoom) = if data.screenshot_ui.is_open() {
                (Point::from((0.0, 0.0)), 1.0)
            } else {
                (cur_camera, cur_zoom)
            };
            let cursor_elements = build_cursor_elements(
                data,
                backend.renderer(),
                cursor_cam,
                cursor_zoom,
                output.current_scale().fractional_scale(),
                1.0,
            );
            let mut age = backend.buffer_age().unwrap_or(0);
            if !data.render.cached_tile_bg.is_empty() && (camera_moved || zoom_changed) {
                age = 0;
            }
            let render_ok = match backend.bind() {
                Ok((renderer, mut framebuffer)) => {
                    let all_elements =
                        crate::render::compose_frame(data, renderer, &output, cursor_elements);
                    let result = damage_tracker.render_output(
                        renderer,
                        &mut framebuffer,
                        age,
                        &all_elements,
                        [0.0f32, 0.0, 0.0, 1.0],
                    );
                    if let Err(err) = result {
                        tracing::warn!("Render error: {err:?}");
                    }

                    // Check if a screenshot was requested
                    if data.pending_screenshot || data.pending_screenshot_screen {
                        let is_screen = data.pending_screenshot_screen;
                        data.pending_screenshot = false;
                        data.pending_screenshot_screen = false;

                        use smithay::backend::renderer::{Bind, Offscreen};
                        let buf_size = output_logical_size(&output)
                            .to_buffer(1, smithay::utils::Transform::Normal);

                        // WITH cursor (all_elements already has cursor)
                        let tex_with =
                            (|| -> Option<smithay::backend::renderer::gles::GlesTexture> {
                                let mut texture = Offscreen::<
                                    smithay::backend::renderer::gles::GlesTexture,
                                >::create_buffer(
                                    renderer,
                                    smithay::backend::allocator::Fourcc::Abgr8888,
                                    buf_size,
                                )
                                .ok()?;
                                let tex_clone = texture.clone();
                                let mut fb = renderer.bind(&mut texture).ok()?;
                                let mut dt =
                                    smithay::backend::renderer::damage::OutputDamageTracker::new(
                                        output_logical_size(&output).to_physical(1),
                                        output.current_scale().fractional_scale(),
                                        output.current_transform(),
                                    );
                                dt.render_output(
                                    renderer,
                                    &mut fb,
                                    0,
                                    &all_elements,
                                    [0.0, 0.0, 0.0, 1.0],
                                )
                                .ok()?;
                                Some(tex_clone)
                            })();

                        // WITHOUT cursor
                        let tex_without =
                            (|| -> Option<smithay::backend::renderer::gles::GlesTexture> {
                                let mut texture = Offscreen::<
                                    smithay::backend::renderer::gles::GlesTexture,
                                >::create_buffer(
                                    renderer,
                                    smithay::backend::allocator::Fourcc::Abgr8888,
                                    buf_size,
                                )
                                .ok()?;
                                let tex_clone = texture.clone();
                                let mut fb = renderer.bind(&mut texture).ok()?;
                                let mut dt =
                                    smithay::backend::renderer::damage::OutputDamageTracker::new(
                                        output_logical_size(&output).to_physical(1),
                                        output.current_scale().fractional_scale(),
                                        output.current_transform(),
                                    );
                                let no_cursor =
                                    crate::render::compose_frame(data, renderer, &output, vec![]);
                                dt.render_output(
                                    renderer,
                                    &mut fb,
                                    0,
                                    &no_cursor,
                                    [0.0, 0.0, 0.0, 1.0],
                                )
                                .ok()?;
                                Some(tex_clone)
                            })();

                        if let (Some(tw), Some(two)) = (tex_with, tex_without) {
                            let default_output = data
                                .focused_output
                                .clone()
                                .unwrap_or_else(|| output.clone());
                            #[allow(clippy::mutable_key_type)]
                            let mut screenshots = std::collections::HashMap::new();
                            screenshots.insert(output.clone(), (tw, two));
                            data.screenshot_ui
                                .open(renderer, screenshots, default_output, false);
                            if is_screen {
                                data.screenshot_ui.select_all();
                                data.pending_screenshot_confirm = true;
                            }
                        }
                    }

                    if data.pending_screenshot_confirm {
                        data.pending_screenshot_confirm = false;
                        if let Ok((size, pixels)) = data.screenshot_ui.capture(renderer) {
                            data.save_screenshot(size, &pixels);
                        }
                        data.restore_pointer_to_canvas();
                        data.screenshot_ui.close();
                    }

                    crate::render::render_screencopy(data, renderer, &output, &all_elements);
                    crate::render::render_capture_frames(data, renderer, &output, &all_elements);
                    true
                }
                Err(err) => {
                    tracing::warn!("Backend bind error: {err:?}");
                    false
                }
            };
            if render_ok && let Err(err) = backend.submit(None) {
                tracing::warn!("Submit error: {err:?}");
            }

            // --- Record camera+zoom for next-frame change detection ---
            {
                let mut os = crate::state::output_state(&output);
                os.last_rendered_camera = os.camera;
                os.last_rendered_zoom = os.zoom;
            }

            // --- Put backend back ---
            data.backend = Some(Backend::Winit(backend));

            // --- Post-render ---
            crate::render::refresh_foreign_toplevels(data);
            crate::render::post_render(data, &output);
            data.display_handle.flush_clients().ok();

            TimeoutAction::ToDuration(Duration::from_millis(16))
        })?;

    Ok(())
}
