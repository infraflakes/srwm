use smithay::{
    utils::{Point, Size},
    wayland::seat::WaylandFocus,
};

use crate::state::Srwc;
use srwc::canvas::{self};
use srwc::config::Action;
use srwc::window_ext::WindowExt;

impl Srwc {
    pub fn execute_action(&mut self, action: &Action) {
        // Snapshot fullscreen window before the guard exits it.
        // Also check gesture_exited_fullscreen (set by exit_fullscreen_for_gesture
        // which runs before execute_action in the gesture path).
        let was_fullscreen = self
            .active_fullscreen()
            .map(|fs| fs.window.clone())
            .or_else(|| self.gestures.exited_fullscreen.take());

        // Any action except ToggleFullscreen/Spawn/ReloadConfig exits fullscreen first
        if self.is_fullscreen()
            && !matches!(
                action,
                Action::ToggleFullscreen | Action::Spawn(_) | Action::ReloadConfig
            )
        {
            self.exit_fullscreen();
        }

        self.with_output_state(|os| os.momentum.stop());
        match action {
            Action::Exec(cmd) => {
                tracing::info!("Spawning: {cmd}");
                crate::state::spawn_command(cmd);
                let now = std::time::Instant::now();
                self.cursor.exec_cursor_show_at = Some(now + std::time::Duration::from_millis(150));
                self.cursor.exec_cursor_deadline = Some(now + std::time::Duration::from_secs(5));
            }
            Action::Spawn(cmd) => {
                tracing::info!("Spawning (no cursor): {cmd}");
                crate::state::spawn_command(cmd);
            }
            Action::CloseWindow => {
                let keyboard = self.keyboard();
                if let Some(focus) = keyboard.current_focus() {
                    let window = self.window_for_surface(&focus.0);
                    if let Some(window) = window {
                        window.send_close();
                    }
                }
            }
            Action::NudgeWindow(dir) => {
                let keyboard = self.keyboard();
                if let Some(focus) = keyboard.current_focus() {
                    if srwc::config::applied_rule(&focus.0).is_some_and(|r| r.widget) {
                        return;
                    }
                    let window = self.window_for_surface(&focus.0);
                    if let Some(window) = window
                        && let Some(loc) = self.space.element_location(&window)
                    {
                        let step = self.config.nav.nudge_step;
                        let (ux, uy) = dir.to_unit_vec();
                        let offset = (
                            (ux * step as f64).round() as i32,
                            (uy * step as f64).round() as i32,
                        );
                        let new_loc = loc + Point::from(offset);
                        self.space.map_element(window, new_loc, false);
                    }
                }
            }
            Action::PanViewport(dir) => {
                self.with_output_state(|os| {
                    os.zoom_target = None;
                    os.zoom_animation_center = None;
                    os.overview_return = None;
                    let zoom = os.zoom;
                    let step = self.config.nav.pan_step / zoom;
                    let (ux, uy) = dir.to_unit_vec();
                    let delta: Point<f64, smithay::utils::Logical> =
                        Point::from((ux * step, uy * step));
                    let base = os.camera_target.unwrap_or(os.camera);
                    os.camera_target = Some(base + delta);
                });
            }
            Action::CenterWindow => {
                let keyboard = self.keyboard();
                let focused_non_widget = keyboard.current_focus().and_then(|focus| {
                    if srwc::config::applied_rule(&focus.0).is_some_and(|r| r.widget) {
                        return None;
                    }
                    self.window_for_surface(&focus.0)
                });
                if let Some(window) = focused_non_widget {
                    self.navigate_to_window(&window, true);
                } else {
                    // No focused non-widget window — find and focus the closest to viewport center
                    let vc = self.usable_center_screen();
                    let (camera, zoom) = self
                        .with_output_state(|os| (os.camera, os.zoom))
                        .unwrap_or_default();
                    let center_x = camera.x + vc.x / zoom;
                    let center_y = camera.y + vc.y / zoom;
                    let closest = self
                        .space
                        .elements()
                        .filter(|w| {
                            !w.wl_surface()
                                .as_ref()
                                .and_then(|s| srwc::config::applied_rule(s))
                                .is_some_and(|r| r.widget)
                        })
                        .min_by(|a, b| {
                            let dist = |w: &smithay::desktop::Window| {
                                let c = self.window_visual_center(w).unwrap_or_default();
                                let dx = c.x - center_x;
                                let dy = c.y - center_y;
                                dx * dx + dy * dy
                            };
                            dist(a).partial_cmp(&dist(b)).unwrap()
                        })
                        .cloned();
                    if let Some(window) = closest {
                        self.navigate_to_window(&window, true);
                    }
                }
            }

            Action::CenterNearest(dir) => {
                #[derive(Clone, PartialEq)]
                enum NavTarget {
                    Window(smithay::desktop::Window),
                }

                let keyboard = self.keyboard();
                let focused = keyboard
                    .current_focus()
                    .and_then(|focus| self.window_for_surface(&focus.0));

                let viewport_size = self.get_viewport_size();
                let vc = self.usable_center_screen();
                let (camera, zoom) = self
                    .with_output_state(|os| (os.camera, os.zoom))
                    .unwrap_or_default();
                let viewport_center = Point::from((camera.x + vc.x / zoom, camera.y + vc.y / zoom));

                let (origin, skip) = if let Some(ref w) = focused {
                    let loc = self.space.element_location(w).unwrap_or_default();
                    let size = w.geometry().size;
                    if canvas::visible_fraction(loc, size, camera, viewport_size, zoom) >= 0.5 {
                        let center = self.window_visual_center(w).unwrap_or_else(|| {
                            Point::from((
                                loc.x as f64 + size.w as f64 / 2.0,
                                loc.y as f64 + size.h as f64 / 2.0,
                            ))
                        });
                        (center, Some(NavTarget::Window(w.clone())))
                    } else {
                        (viewport_center, None)
                    }
                } else {
                    (viewport_center, None)
                };

                let windows = self
                    .space
                    .elements()
                    .filter(|w| {
                        !w.wl_surface()
                            .as_ref()
                            .and_then(|s| srwc::config::applied_rule(s))
                            .is_some_and(|r| r.widget)
                    })
                    .map(|w| {
                        let loc = self.space.element_location(w).unwrap_or_default();
                        let size = w.geometry().size;
                        let closest = canvas::closest_point_on_rect(origin, loc, size);
                        let point = if closest == origin {
                            self.window_visual_center(w).unwrap_or_else(|| {
                                Point::from((
                                    loc.x as f64 + size.w as f64 / 2.0,
                                    loc.y as f64 + size.h as f64 / 2.0,
                                ))
                            })
                        } else {
                            closest
                        };
                        (NavTarget::Window(w.clone()), point)
                    });

                let nearest = canvas::find_nearest(origin, dir, windows, skip.as_ref());
                match nearest {
                    Some(NavTarget::Window(w)) => {
                        self.navigate_to_window(&w, false);
                    }
                    None => {}
                }
            }
            Action::CycleWindows { backward } => {
                if self.focus.history.is_empty() {
                    return;
                }

                let len = self.focus.history.len();
                if let Some(ref mut idx) = self.focus.cycle_index {
                    if *backward {
                        *idx = (*idx + len - 1) % len;
                    } else {
                        *idx = (*idx + 1) % len;
                    }
                } else {
                    // First Tab press: jump to previous window (index 1)
                    self.focus.cycle_index = Some(1 % len);
                }

                let idx = self.focus.cycle_index.unwrap();
                if let Some(window) = self.focus.history.get(idx).cloned() {
                    self.navigate_to_window(&window, false);
                }
            }
            Action::ZoomIn => {
                let new_zoom = self
                    .with_output_state(|os| (os.zoom * self.config.zoom.step).min(canvas::MAX_ZOOM))
                    .unwrap_or(1.0);
                let new_zoom = canvas::snap_zoom(new_zoom);
                self.zoom_to_anchored(new_zoom);
            }
            Action::ZoomOut => {
                let new_zoom = self
                    .with_output_state(|os| (os.zoom / self.config.zoom.step).max(self.min_zoom()))
                    .unwrap_or(1.0);
                let new_zoom = canvas::snap_zoom(new_zoom);
                self.zoom_to_anchored(new_zoom);
            }
            Action::ZoomReset => {
                self.zoom_to_anchored(1.0);
            }
            Action::ZoomToFit => {
                self.with_output_state(|os| {
                    let overview_ret = os.overview_return.take();
                    if let Some((saved_camera, saved_zoom)) = overview_ret {
                        // Toggle back from overview
                        let vc = if let Some(output) = self.active_output() {
                            crate::state::usable_center_for_output(&output)
                        } else {
                            Point::default()
                        };
                        os.zoom_animation_center = Some(Point::from((
                            saved_camera.x + vc.x / saved_zoom,
                            saved_camera.y + vc.y / saved_zoom,
                        )));
                        os.camera_target = Some(saved_camera);
                        os.zoom_target = Some(saved_zoom);
                    } else {
                        // Compute bounding box of all windows
                        let viewport = if let Some(output) = self.active_output() {
                            crate::state::output_logical_size(&output)
                        } else {
                            Size::default()
                        };
                        let vc = if let Some(output) = self.active_output() {
                            crate::state::usable_center_for_output(&output)
                        } else {
                            Point::default()
                        };
                        let windows = self
                            .space
                            .elements()
                            .filter(|w| {
                                !w.wl_surface()
                                    .as_ref()
                                    .and_then(|s| srwc::config::applied_rule(s))
                                    .is_some_and(|r| r.widget)
                            })
                            .map(|w| {
                                let loc = self.space.element_location(w).unwrap_or_default();
                                let size = w.geometry().size;
                                (loc, size)
                            });
                        let bbox = canvas::all_windows_bbox(windows);
                        if let Some(bbox) = bbox {
                            let fit_zoom =
                                canvas::zoom_to_fit(bbox, viewport, self.config.zoom.fit_padding);
                            let bbox_cx = bbox.loc.x as f64 + bbox.size.w as f64 / 2.0;
                            let bbox_cy = bbox.loc.y as f64 + bbox.size.h as f64 / 2.0;
                            let new_camera: Point<f64, smithay::utils::Logical> =
                                Point::from((bbox_cx - vc.x / fit_zoom, bbox_cy - vc.y / fit_zoom));
                            os.overview_return = Some((os.camera, os.zoom));
                            os.zoom_animation_center = Some(Point::from((bbox_cx, bbox_cy)));
                            os.camera_target = Some(new_camera);
                            os.zoom_target = Some(fit_zoom);
                        }
                    }
                });
            }
            Action::ToggleFullscreen => {
                if self.is_fullscreen() {
                    self.exit_fullscreen();
                } else if was_fullscreen.is_some() {
                    // Gesture already exited fullscreen — don't re-enter
                } else {
                    let keyboard = self.keyboard();
                    if let Some(focus) = keyboard.current_focus() {
                        let window = self.window_for_surface(&focus.0);
                        if let Some(window) = window {
                            self.enter_fullscreen(&window);
                        }
                    }
                }
            }
            Action::FitWindow => {
                let keyboard = self.keyboard();
                if let Some(focus) = keyboard.current_focus() {
                    let window = self.window_for_surface(&focus.0);
                    if let Some(window) = window {
                        self.toggle_fit_window(&window);
                    }
                }
            }
            Action::SendToOutput(dir) => {
                let keyboard = self.keyboard();
                if let Some(focus) = keyboard.current_focus() {
                    if srwc::config::applied_rule(&focus.0).is_some_and(|r| r.widget) {
                        return;
                    }
                    let window = self.window_for_surface(&focus.0);
                    if let Some(window) = window
                        && let Some(from_output) = self.output_for_window(&window)
                        && let Some(target_output) = self.output_in_direction(&from_output, dir)
                    {
                        // Compute target output's usable area center in canvas coords
                        let (target_cam, target_zoom) = {
                            let os = crate::state::output_state(&target_output);
                            (os.camera, os.zoom)
                        };
                        let target_vc = crate::state::usable_center_for_output(&target_output);
                        let center_x = target_cam.x + target_vc.x / target_zoom;
                        let center_y = target_cam.y + target_vc.y / target_zoom;
                        let geo = window.geometry();
                        let new_loc = Point::from((
                            (center_x - geo.size.w as f64 / 2.0) as i32,
                            (center_y - geo.size.h as f64 / 2.0) as i32,
                        ));
                        self.space.map_element(window.clone(), new_loc, true);
                        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                        self.raise_and_focus(&window, serial);
                    }
                }
            }
            Action::ReloadConfig => {
                self.reload_config();
            }
            Action::Quit => {
                tracing::info!("Quit action triggered — stopping compositor");
                self.save_cameras();
                self.loop_signal.stop();
            }
            Action::Screenshot => {
                // Open interactive screenshot UI — capture happens in render loop
                // where the renderer is available.
                self.screenshot.pending = true;
            }
            Action::ScreenshotScreen => {
                self.screenshot.pending_screen = true;
            }
            Action::ConfirmScreenshot => {
                self.confirm_screenshot();
            }
            Action::CancelScreenshot => {
                self.restore_pointer_to_canvas();
                self.screenshot.ui.close();
            }
            Action::ScreenshotTogglePointer => {
                self.screenshot.ui.toggle_pointer();
            }
        }
    }

    fn zoom_to_anchored(&mut self, target_zoom: f64) {
        let vc = self.usable_center_screen();
        self.with_output_state(|os| {
            os.overview_return = None;
            let camera = os.camera;
            let zoom = os.zoom;
            let vc_canvas = Point::from((camera.x + vc.x / zoom, camera.y + vc.y / zoom));
            let new_camera = canvas::zoom_anchor_camera(vc_canvas, vc, target_zoom);
            os.zoom_animation_center = Some(vc_canvas);
            os.zoom_target = Some(target_zoom);
            os.camera_target = Some(new_camera);
        });
    }
}
