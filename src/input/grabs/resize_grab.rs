use std::cell::RefCell;

use smithay::{
    desktop::Window,
    input::{
        SeatHandler,
        pointer::{ButtonEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle},
    },
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Size},
    wayland::{compositor::with_states, seat::WaylandFocus},
};

use smithay::input::pointer::CursorImageStatus;

use crate::state::Srwc;
use srwc::canvas::{self, CanvasPos, canvas_to_screen};

/// Tracks the resize lifecycle for a window. Stored in the surface data map
/// (wrapped in `RefCell`) so that `compositor::commit()` can reposition
/// top/left-edge resizes.
#[derive(Default, Clone, Copy)]
pub enum ResizeState {
    #[default]
    Idle,
    Resizing {
        edges: xdg_toplevel::ResizeEdge,
        initial_window_location: Point<i32, Logical>,
        initial_window_size: Size<i32, Logical>,
    },
    WaitingForLastCommit {
        edges: xdg_toplevel::ResizeEdge,
        initial_window_location: Point<i32, Logical>,
        initial_window_size: Size<i32, Logical>,
    },
}

pub struct ResizeSurfaceGrab {
    pub start_data: GrabStartData<Srwc>,
    pub window: Window,
    pub edges: xdg_toplevel::ResizeEdge,
    pub initial_window_location: Point<i32, Logical>,
    pub initial_window_size: Size<i32, Logical>,
    pub last_window_size: Size<i32, Logical>,
    pub output: Output,
    pub last_clamped_location: Point<f64, Logical>,
    /// Throttle X11 configures to avoid overwhelming the client (X11 redraws synchronously).
    pub last_x11_configure: Option<std::time::Instant>,
}

/// Check if `edges` includes a horizontal/vertical component via raw bit values.
/// ResizeEdge values: Top=1, Bottom=2, Left=4, Right=8, combinations are ORed.
pub fn has_top(edges: xdg_toplevel::ResizeEdge) -> bool {
    edges as u32 & 1 != 0
}
pub fn has_bottom(edges: xdg_toplevel::ResizeEdge) -> bool {
    edges as u32 & 2 != 0
}
pub fn has_left(edges: xdg_toplevel::ResizeEdge) -> bool {
    edges as u32 & 4 != 0
}
pub fn has_right(edges: xdg_toplevel::ResizeEdge) -> bool {
    edges as u32 & 8 != 0
}

/// Calculate new dimensions based on resize edges and pointer/gesture delta.
pub fn compute_resize(
    edges: xdg_toplevel::ResizeEdge,
    initial_size: Size<i32, Logical>,
    delta: Point<f64, Logical>,
) -> (i32, i32) {
    let mut new_w = initial_size.w;
    let mut new_h = initial_size.h;

    if has_left(edges) {
        new_w -= delta.x as i32;
    } else if has_right(edges) {
        new_w += delta.x as i32;
    }
    if has_top(edges) {
        new_h -= delta.y as i32;
    } else if has_bottom(edges) {
        new_h += delta.y as i32;
    }

    (new_w.max(1), new_h.max(1))
}

/// Send resize configuration to toplevel/X11 window with throttling.
pub fn send_resize_configure(
    window: &Window,
    new_size: Size<i32, Logical>,
    last_size: &mut Size<i32, Logical>,
    last_x11_configure: &mut Option<std::time::Instant>,
) -> bool {
    if new_size != *last_size {
        *last_size = new_size;

        if let Some(toplevel) = window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.size = Some(new_size);
                state.states.set(xdg_toplevel::State::Resizing);
            });
            toplevel.send_pending_configure();
            return true;
        } else if let Some(x11) = window.x11_surface() {
            let now = std::time::Instant::now();
            let throttle_ok = last_x11_configure
                .as_ref()
                .is_none_or(|t| now.duration_since(*t) >= std::time::Duration::from_millis(16));
            if throttle_ok {
                *last_x11_configure = Some(now);
                let mut geo = x11.geometry();
                geo.size = new_size;
                let _ = x11.configure(geo);
                return true;
            }
        }
    }
    false
}

impl PointerGrab<Srwc> for ResizeSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut Srwc,
        handle: &mut PointerInnerHandle<'_, Srwc>,
        _focus: Option<(<Srwc as SeatHandler>::PointerFocus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        // Force pointer back if Phase 3 input routing crossed to another output.
        // event.location is in the wrong canvas space — use last valid position.
        if data
            .focused_output
            .as_ref()
            .is_some_and(|fo| *fo != self.output)
        {
            data.focused_output = Some(self.output.clone());
            let clamped_event = MotionEvent {
                location: self.last_clamped_location,
                serial: event.serial,
                time: event.time,
            };
            handle.motion(data, None, &clamped_event);
            return;
        }

        // Clamp pointer to the grab's output bounds
        let (camera, zoom) = {
            let os = crate::state::output_state(&self.output);
            (os.camera, os.zoom)
        };
        let output_size = crate::state::output_logical_size(&self.output);
        let screen = canvas_to_screen(CanvasPos(event.location), camera, zoom).0;
        let clamped_screen: Point<f64, Logical> = (
            screen.x.clamp(0.0, output_size.w as f64 - 1.0),
            screen.y.clamp(0.0, output_size.h as f64 - 1.0),
        )
            .into();
        let clamped = canvas::screen_to_canvas(canvas::ScreenPos(clamped_screen), camera, zoom).0;
        self.last_clamped_location = clamped;

        let delta = clamped - self.start_data.location;

        let (new_w, new_h) = compute_resize(self.edges, self.initial_window_size, delta);

        let new_size = Size::from((new_w, new_h));
        send_resize_configure(
            &self.window,
            new_size,
            &mut self.last_window_size,
            &mut self.last_x11_configure,
        );

        // Warp pointer to clamped position so it visually stops at output edge
        let clamped_event = MotionEvent {
            location: clamped,
            serial: event.serial,
            time: event.time,
        };
        handle.motion(data, None, &clamped_event);
    }

    fn button(
        &mut self,
        data: &mut Srwc,
        handle: &mut PointerInnerHandle<'_, Srwc>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            // Grab released — unset Resizing state (Wayland only) and
            // transition to WaitingForLastCommit for position adjustment
            if let Some(toplevel) = self.window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                });
                toplevel.send_pending_configure();
            } else if let Some(x11) = self.window.x11_surface() {
                let mut geo = x11.geometry();
                geo.size = self.last_window_size;
                x11.configure(geo).ok();
            }

            let Some(surface) = self.window.wl_surface().map(|s| s.into_owned()) else {
                handle.unset_grab(self, data, event.serial, event.time, true);
                return;
            };
            let edges = self.edges;
            let initial_window_location = self.initial_window_location;
            let initial_window_size = self.initial_window_size;
            with_states(&surface, |states| {
                states
                    .data_map
                    .get_or_insert(|| RefCell::new(ResizeState::Idle))
                    .replace(ResizeState::WaitingForLastCommit {
                        edges,
                        initial_window_location,
                        initial_window_size,
                    });
            });

            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn unset(&mut self, data: &mut Srwc) {
        data.cursor.grab_cursor = false;
        data.cursor.cursor_status = CursorImageStatus::default_named();
    }

    crate::input::grabs::forward_pointer_grab_methods!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn sz(w: i32, h: i32) -> Size<i32, Logical> {
        Size::from((w, h))
    }
    fn delta(x: f64, y: f64) -> Point<f64, Logical> {
        Point::from((x, y))
    }

    // ── Edge detection ───────────────────────────────────────────────

    #[test]
    fn top_edge() {
        assert!(has_top(xdg_toplevel::ResizeEdge::Top));
        assert!(has_top(xdg_toplevel::ResizeEdge::TopLeft));
        assert!(has_top(xdg_toplevel::ResizeEdge::TopRight));
        assert!(!has_top(xdg_toplevel::ResizeEdge::Bottom));
        assert!(!has_top(xdg_toplevel::ResizeEdge::Left));
        assert!(!has_top(xdg_toplevel::ResizeEdge::Right));
    }

    #[test]
    fn bottom_edge() {
        assert!(has_bottom(xdg_toplevel::ResizeEdge::Bottom));
        assert!(has_bottom(xdg_toplevel::ResizeEdge::BottomLeft));
        assert!(has_bottom(xdg_toplevel::ResizeEdge::BottomRight));
        assert!(!has_bottom(xdg_toplevel::ResizeEdge::Top));
    }

    #[test]
    fn left_edge() {
        assert!(has_left(xdg_toplevel::ResizeEdge::Left));
        assert!(has_left(xdg_toplevel::ResizeEdge::TopLeft));
        assert!(has_left(xdg_toplevel::ResizeEdge::BottomLeft));
        assert!(!has_left(xdg_toplevel::ResizeEdge::Right));
    }

    #[test]
    fn right_edge() {
        assert!(has_right(xdg_toplevel::ResizeEdge::Right));
        assert!(has_right(xdg_toplevel::ResizeEdge::TopRight));
        assert!(has_right(xdg_toplevel::ResizeEdge::BottomRight));
        assert!(!has_right(xdg_toplevel::ResizeEdge::Left));
    }

    // ── compute_resize ───────────────────────────────────────────────

    #[test]
    fn resize_right_increases_width() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::Right,
            sz(200, 100),
            delta(50.0, 0.0),
        );
        assert_eq!(w, 250);
        assert_eq!(h, 100);
    }

    #[test]
    fn resize_left_increases_width_on_negative_delta() {
        // Dragging left (negative x) on the left edge should grow the window
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::Left,
            sz(200, 100),
            delta(-50.0, 0.0),
        );
        assert_eq!(w, 250);
        assert_eq!(h, 100);
    }

    #[test]
    fn resize_bottom_increases_height() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::Bottom,
            sz(200, 100),
            delta(0.0, 30.0),
        );
        assert_eq!(w, 200);
        assert_eq!(h, 130);
    }

    #[test]
    fn resize_top_increases_height_on_negative_delta() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::Top,
            sz(200, 100),
            delta(0.0, -40.0),
        );
        assert_eq!(w, 200);
        assert_eq!(h, 140);
    }

    #[test]
    fn resize_bottom_right_both_axes() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::BottomRight,
            sz(200, 100),
            delta(10.0, 20.0),
        );
        assert_eq!(w, 210);
        assert_eq!(h, 120);
    }

    #[test]
    fn resize_top_left_both_axes() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::TopLeft,
            sz(200, 100),
            delta(-15.0, -25.0),
        );
        assert_eq!(w, 215);
        assert_eq!(h, 125);
    }

    #[test]
    fn resize_clamps_to_minimum_1x1() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::Right,
            sz(10, 10),
            delta(-500.0, -500.0),
        );
        assert_eq!(w, 1, "width should clamp to 1");
        assert_eq!(h, 10, "height unchanged (right edge doesn't affect height)");
    }

    #[test]
    fn resize_clamps_both_axes() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::BottomRight,
            sz(5, 5),
            delta(-100.0, -100.0),
        );
        assert_eq!(w, 1);
        assert_eq!(h, 1);
    }

    #[test]
    fn resize_none_edge_no_change() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::None,
            sz(200, 100),
            delta(50.0, 50.0),
        );
        assert_eq!(w, 200);
        assert_eq!(h, 100);
    }

    #[test]
    fn resize_zero_delta_no_change() {
        let (w, h) = compute_resize(
            xdg_toplevel::ResizeEdge::BottomRight,
            sz(300, 200),
            delta(0.0, 0.0),
        );
        assert_eq!(w, 300);
        assert_eq!(h, 200);
    }

    #[test]
    fn resize_fractional_delta_truncates() {
        // 200 + 0.9 as i32 = 200 (truncation, not rounding)
        let (w, _) = compute_resize(
            xdg_toplevel::ResizeEdge::Right,
            sz(200, 100),
            delta(0.9, 0.0),
        );
        assert_eq!(w, 200);
    }
}
