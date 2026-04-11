pub mod compositor;
pub mod layer_shell;
pub mod xdg_shell;
pub mod xwayland;

use crate::state::{FocusTarget, Srwc};
use smithay::input::dnd::{DnDGrab, DndGrabHandler};
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::shell::xdg::dialog::XdgDialogHandler;
use smithay::{
    backend::renderer::ImportDma,
    delegate_cursor_shape, delegate_data_control, delegate_data_device, delegate_dmabuf,
    delegate_ext_data_control, delegate_fractional_scale, delegate_idle_inhibit,
    delegate_keyboard_shortcuts_inhibit, delegate_output, delegate_pointer_constraints,
    delegate_pointer_gestures, delegate_presentation, delegate_primary_selection,
    delegate_relative_pointer, delegate_seat, delegate_single_pixel_buffer, delegate_viewporter,
    delegate_xdg_activation,
    input::{
        Seat, SeatHandler, SeatState, keyboard,
        pointer::{CursorIcon, CursorImageStatus, PointerHandle},
    },
    reexports::input::DeviceCapability as LibinputCapability,
    reexports::wayland_server::{
        Resource,
        protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    },
    utils::{Logical, Point, Rectangle},
    wayland::{
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        fractional_scale::FractionalScaleHandler,
        idle_inhibit::IdleInhibitHandler,
        keyboard_shortcuts_inhibit::{KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitor},
        output::OutputHandler,
        pointer_constraints::PointerConstraintsHandler,
        selection::{
            SelectionHandler,
            data_device::{
                DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler, set_data_device_focus,
            },
            ext_data_control::{
                DataControlHandler as ExtDataControlHandler,
                DataControlState as ExtDataControlState,
            },
            primary_selection::{
                PrimarySelectionHandler, PrimarySelectionState, set_primary_focus,
            },
            wlr_data_control::{DataControlHandler, DataControlState},
        },
        tablet_manager::TabletSeatHandler,
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
};
use srwc::window_ext::WindowExt;

impl SeatHandler for Srwc {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;
    type TouchFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        // During a compositor grab (pan, resize) or decoration hover,
        // we control the cursor. Ignore client updates.
        if self.cursor.grab_cursor || self.cursor.decoration_cursor {
            return;
        }
        // During exec loading (after grace period), replace default cursor with
        // Wait but let client surface cursors through (they take priority).
        if self.cursor.exec_cursor_deadline.is_some()
            && self
                .cursor
                .exec_cursor_show_at
                .is_none_or(|t| std::time::Instant::now() >= t)
            && matches!(&image, CursorImageStatus::Named(icon) if *icon == CursorIcon::Default)
        {
            self.cursor.cursor_status = CursorImageStatus::Named(CursorIcon::Wait);
            return;
        }
        self.cursor.cursor_status = image;
    }

    fn led_state_changed(&mut self, _seat: &Seat<Self>, led_state: keyboard::LedState) {
        for device in self
            .session_ctx
            .input_devices
            .iter_mut()
            .filter(|d| d.has_capability(LibinputCapability::Keyboard))
        {
            device.led_update(led_state.into());
        }
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|f| dh.get_client(f.0.id()).ok());
        set_data_device_focus(dh, seat, client.clone());
        set_primary_focus(dh, seat, client);

        // Update focus history (skip during Alt-Tab cycling — history is frozen)
        if self.cycle_state.is_none()
            && let Some(focus) = focused
        {
            self.update_focus_history(&focus.0);
        }
    }
}

delegate_seat!(Srwc);

impl SelectionHandler for Srwc {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Srwc {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}

impl DndGrabHandler for Srwc {
    fn dropped(
        &mut self,
        _target: Option<smithay::input::dnd::DndTarget<'_, Self>>,
        _validated: bool,
        _seat: Seat<Self>,
        _location: Point<f64, Logical>,
    ) {
        self.dnd_icon = None;
        self.mark_all_dirty();
    }
}

impl WaylandDndGrabHandler for Srwc {
    fn dnd_requested<S: smithay::input::dnd::Source>(
        &mut self,
        source: S,
        icon: Option<WlSurface>,
        seat: Seat<Self>,
        serial: smithay::utils::Serial,
        type_: smithay::input::dnd::GrabType,
    ) {
        self.dnd_icon = icon.map(|surface| crate::state::DndIcon {
            surface,
            offset: Point::new(0, 0),
        });

        match type_ {
            smithay::input::dnd::GrabType::Pointer => {
                let Some(pointer) = seat.get_pointer() else {
                    tracing::warn!("DnD requested with Pointer type but seat has no pointer");
                    return;
                };
                let Some(start_data) = pointer.grab_start_data() else {
                    tracing::warn!("DnD requested but pointer grab start data is unavailable");
                    return;
                };
                let grab = DnDGrab::new_pointer(&self.display_handle, start_data, source, seat);
                pointer.set_grab(self, grab, serial, smithay::input::pointer::Focus::Keep);
            }
            smithay::input::dnd::GrabType::Touch => {
                let Some(touch) = seat.get_touch() else {
                    tracing::warn!("DnD requested with Touch type but seat has no touch device");
                    return;
                };
                let Some(start_data) = touch.grab_start_data() else {
                    tracing::warn!("DnD requested but touch grab start data is unavailable");
                    return;
                };
                let grab = DnDGrab::new_touch(&self.display_handle, start_data, source, seat);
                touch.set_grab(self, grab, serial);
            }
        }

        self.mark_all_dirty();
    }
}

delegate_data_device!(Srwc);

impl OutputHandler for Srwc {}

delegate_output!(Srwc);

impl TabletSeatHandler for Srwc {}

delegate_cursor_shape!(Srwc);

impl DmabufHandler for Srwc {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: smithay::backend::allocator::dmabuf::Dmabuf,
        notifier: ImportNotifier,
    ) {
        let Some(backend) = self.backend.as_mut() else {
            notifier.failed();
            return;
        };
        if backend.renderer().import_dmabuf(&dmabuf, None).is_ok() {
            let _ = notifier.successful::<Srwc>();
        } else {
            notifier.failed();
        }
    }
}

delegate_dmabuf!(Srwc);

delegate_viewporter!(Srwc);

impl FractionalScaleHandler for Srwc {
    fn new_fractional_scale(&mut self, _surface: WlSurface) {}
}

delegate_fractional_scale!(Srwc);

impl XdgActivationHandler for Srwc {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn token_created(&mut self, _token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        if data.serial.is_some() {
            let now = std::time::Instant::now();
            self.cursor.exec_cursor_show_at = Some(now + std::time::Duration::from_millis(150));
            self.cursor.exec_cursor_deadline = Some(now + std::time::Duration::from_secs(5));
        }
        true
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        // Same client activating itself (e.g. Telegram switching chats) — cancel loading cursor
        if let Some(req_surface) = &token_data.surface {
            let req_client = self.display_handle.get_client(req_surface.id()).ok();
            let act_client = self.display_handle.get_client(surface.id()).ok();
            if req_client.is_some() && req_client == act_client {
                self.cursor.exec_cursor_show_at = None;
                self.cursor.exec_cursor_deadline = None;
            }
        }

        // Only honor tokens created from user input (has a serial).
        // Tokens without a serial are spontaneous attention requests from
        // background apps — ignore those to prevent focus stealing.
        if token_data.serial.is_none() {
            return;
        }
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&surface))
            .cloned();
        if let Some(window) = window {
            // Skip windows that haven't rendered yet — navigate_to_window on a
            // zero-sized window sets a fractional camera that breaks cascade.
            if window.geometry().size.w == 0 || window.geometry().size.h == 0 {
                return;
            }
            let mostly_visible = self.space.element_location(&window).is_some_and(|loc| {
                let (camera, zoom) = self
                    .with_output_state(|os| (os.camera, os.zoom))
                    .unwrap_or_default();
                srwc::canvas::visible_fraction(
                    loc,
                    window.geometry().size,
                    camera,
                    self.get_viewport_size(),
                    zoom,
                ) >= 0.5
            });
            if mostly_visible {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.raise_and_focus(&window, serial);
            } else {
                self.navigate_to_window(&window, true);
            }
        }
    }
}

delegate_xdg_activation!(Srwc);

impl PrimarySelectionHandler for Srwc {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.primary_selection_state
    }
}

delegate_primary_selection!(Srwc);

impl DataControlHandler for Srwc {
    fn data_control_state(&mut self) -> &mut DataControlState {
        &mut self.data_control_state
    }
}

delegate_data_control!(Srwc);

impl ExtDataControlHandler for Srwc {
    fn data_control_state(&mut self) -> &mut ExtDataControlState {
        &mut self.ext_data_control_state
    }
}

delegate_ext_data_control!(Srwc);

impl PointerConstraintsHandler for Srwc {
    fn new_constraint(&mut self, _surface: &WlSurface, _pointer: &PointerHandle<Self>) {
        self.maybe_activate_pointer_constraint();
    }

    fn cursor_position_hint(
        &mut self,
        surface: &WlSurface,
        pointer: &PointerHandle<Self>,
        location: Point<f64, Logical>,
    ) {
        use smithay::wayland::pointer_constraints::with_pointer_constraint;

        let is_active =
            with_pointer_constraint(surface, pointer, |c| c.is_some_and(|c| c.is_active()));
        if !is_active {
            return;
        }

        // location is surface-local. Find the surface's canvas origin to convert.
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(surface))
            .cloned();
        if let Some(window) = window
            && let Some(loc) = self.space.element_location(&window)
        {
            self.cursor.pointer_position_hint = Some(loc.to_f64() + location);
        }
    }
}

delegate_pointer_constraints!(Srwc);

delegate_relative_pointer!(Srwc);
delegate_pointer_gestures!(Srwc);

impl KeyboardShortcutsInhibitHandler for Srwc {
    fn keyboard_shortcuts_inhibit_state(
        &mut self,
    ) -> &mut smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState {
        &mut self.keyboard_shortcuts_inhibit_state
    }

    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        inhibitor.activate();
    }

    fn inhibitor_destroyed(&mut self, _inhibitor: KeyboardShortcutsInhibitor) {}
}

delegate_keyboard_shortcuts_inhibit!(Srwc);

use smithay::delegate_input_method_manager;
use smithay::delegate_text_input_manager;
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface};

impl InputMethodHandler for Srwc {
    fn new_popup(&mut self, surface: PopupSurface) {
        if let Err(err) = self
            .popups
            .track_popup(smithay::desktop::PopupKind::from(surface))
        {
            tracing::warn!("Failed to track IME popup: {}", err);
        }
    }

    fn popup_repositioned(&mut self, _: PopupSurface) {}

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        if let Some(parent) = surface.get_parent().map(|parent| parent.surface.clone()) {
            let _ = smithay::desktop::PopupManager::dismiss_popup(
                &parent,
                &smithay::desktop::PopupKind::from(surface),
            );
        }
    }

    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.space
            .elements()
            .find_map(|window| {
                (window.wl_surface().as_deref() == Some(parent)).then(|| window.geometry())
            })
            .unwrap_or_default()
    }
}

delegate_text_input_manager!(Srwc);
delegate_input_method_manager!(Srwc);
use smithay::delegate_virtual_keyboard_manager;
delegate_virtual_keyboard_manager!(Srwc);

impl IdleInhibitHandler for Srwc {
    fn inhibit(&mut self, _surface: WlSurface) {}
    fn uninhibit(&mut self, _surface: WlSurface) {}
}

delegate_idle_inhibit!(Srwc);

use smithay::delegate_idle_notify;
use smithay::wayland::idle_notify::{IdleNotifierHandler, IdleNotifierState};

impl IdleNotifierHandler for Srwc {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<Self> {
        &mut self.idle_notifier_state
    }
}
delegate_idle_notify!(Srwc);

delegate_presentation!(Srwc);
delegate_single_pixel_buffer!(Srwc);

use smithay::delegate_xdg_foreign;
use smithay::wayland::xdg_foreign::{XdgForeignHandler, XdgForeignState};

impl XdgForeignHandler for Srwc {
    fn xdg_foreign_state(&mut self) -> &mut XdgForeignState {
        &mut self.xdg_foreign_state
    }
}
delegate_xdg_foreign!(Srwc);

use smithay::delegate_content_type;
delegate_content_type!(Srwc);

use smithay::delegate_xdg_dialog;

use smithay::wayland::shell::xdg::dialog::ToplevelDialogHint;

impl XdgDialogHandler for Srwc {
    fn dialog_hint_changed(&mut self, toplevel: ToplevelSurface, hint: ToplevelDialogHint) {
        if hint == ToplevelDialogHint::Modal {
            let wl_surface = toplevel.wl_surface().clone();
            let window = self.window_for_surface(&wl_surface);
            if let Some(window) = window {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                self.raise_and_focus(&window, serial);
            }
        }
    }
}
delegate_xdg_dialog!(Srwc);

use smithay::delegate_xdg_decoration;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;

impl XdgDecorationHandler for Srwc {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        // CSD-first: tell client to draw its own decorations
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }

    fn request_mode(
        &mut self,
        toplevel: ToplevelSurface,
        mode: smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    ) {
        use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;

        let wl_surface = toplevel.wl_surface().clone();

        // If a window rule forces decoration mode, override the client's request
        let effective_mode = if let Some(rule) = srwc::config::applied_rule(&wl_surface)
            && rule.decoration != srwc::config::DecorationMode::Client
        {
            Mode::ServerSide
        } else {
            mode
        };

        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(effective_mode);
        });
        toplevel.send_configure();

        if effective_mode == Mode::ServerSide {
            self.pending_ssd.insert(wl_surface.id());
            // If the window is already mapped (request_mode came after first commit),
            // create the SSD decoration immediately.
            let window = self
                .space
                .elements()
                .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
                .cloned();
            if let Some(window) = window {
                let geo = window.geometry();
                if geo.size.w > 0 && !self.decorations.contains_key(&wl_surface.id()) {
                    let deco = crate::decorations::WindowDecoration::new(
                        geo.size.w,
                        true,
                        &self.config.decorations,
                    );
                    self.decorations.insert(wl_surface.id(), deco);
                }
            }
        } else {
            self.pending_ssd.remove(&wl_surface.id());
            self.decorations.remove(&wl_surface.id());
            self.render.csd_shadows.remove(&wl_surface.id());
        }
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }
}

delegate_xdg_decoration!(Srwc);

use srwc::protocols::foreign_toplevel::{ForeignToplevelHandler, ForeignToplevelManagerState};

impl ForeignToplevelHandler for Srwc {
    fn foreign_toplevel_manager_state(&mut self) -> &mut ForeignToplevelManagerState {
        &mut self.foreign_toplevel_state
    }

    fn foreign_toplevel_outputs(&self) -> Vec<smithay::output::Output> {
        self.space.outputs().cloned().collect()
    }

    fn activate(&mut self, wl_surface: WlSurface) {
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
            .cloned();
        if let Some(window) = window {
            self.navigate_to_window(&window, true);
        }
    }

    fn close(&mut self, wl_surface: WlSurface) {
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
            .cloned();
        if let Some(window) = window {
            window.send_close();
        }
    }

    fn set_fullscreen(&mut self, wl_surface: WlSurface, _wl_output: Option<WlOutput>) {
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
            .cloned();
        if let Some(window) = window {
            self.enter_fullscreen(&window);
        }
    }

    fn unset_fullscreen(&mut self, wl_surface: WlSurface) {
        if let Some(output) = self.find_fullscreen_output_for_surface(&wl_surface) {
            self.exit_fullscreen_on(&output);
        }
    }

    fn set_maximized(&mut self, wl_surface: WlSurface) {
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
            .cloned();
        if let Some(window) = window {
            self.toggle_fit_window(&window);
        }
    }

    fn unset_maximized(&mut self, wl_surface: WlSurface) {
        let window = self
            .space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(&wl_surface))
            .cloned();
        if let Some(window) = window {
            self.unfit_window(&window);
        }
    }
}

srwc::delegate_foreign_toplevel!(Srwc);

use srwc::protocols::screencopy::{Screencopy, ScreencopyHandler, ScreencopyManagerState};

impl ScreencopyHandler for Srwc {
    fn frame(&mut self, screencopy: Screencopy) {
        self.pending_screencopies.push(screencopy);
    }

    fn screencopy_state(&mut self) -> &mut ScreencopyManagerState {
        &mut self.screencopy_state
    }
}

srwc::delegate_screencopy!(Srwc);

srwc::delegate_image_capture_source!(Srwc);

use srwc::protocols::image_copy_capture::{
    ImageCopyCaptureHandler, ImageCopyCaptureState, PendingCapture,
};

impl ImageCopyCaptureHandler for Srwc {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.image_copy_capture_state
    }

    fn capture_frame(&mut self, capture: PendingCapture) {
        self.pending_captures.push(capture);
    }
}

srwc::delegate_image_copy_capture!(Srwc);

use srwc::protocols::output_management::{
    OutputManagementHandler, OutputManagementState, RequestedHeadConfig,
};

impl OutputManagementHandler for Srwc {
    fn output_management_state(&mut self) -> &mut OutputManagementState {
        &mut self.output_management_state
    }

    fn apply_output_config(&mut self, configs: Vec<RequestedHeadConfig>) -> bool {
        for cfg in &configs {
            let output = self
                .space
                .outputs()
                .find(|o| o.name() == cfg.output_name)
                .cloned();
            let Some(output) = output else {
                return false;
            };

            let current_mode = output.current_mode();
            let new_transform = cfg.transform.or_else(|| Some(output.current_transform()));
            let new_scale = cfg.scale.map(smithay::output::Scale::Fractional);

            let new_position = cfg.position.map(|(x, y)| {
                let mut os = crate::state::output_state(&output);
                os.layout_position = (x, y).into();
                os.layout_position
            });

            output.change_current_state(current_mode, new_transform, new_scale, new_position);

            self.render.cached_bg_elements.remove(&cfg.output_name);
            self.remove_capture_state(&cfg.output_name);
        }
        self.mark_all_dirty();
        self.output_config_dirty = true;
        true
    }
}

srwc::delegate_output_management!(Srwc);

use crate::state::SessionLock;
use smithay::delegate_session_lock;
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker,
};

impl SessionLockHandler for Srwc {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_manager_state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        tracing::info!("Session lock requested");
        self.session_lock = SessionLock::Pending(confirmation);

        // Kill all transient input/animation state so nothing fires during lock
        self.gestures.state = None;
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            let mut os = crate::state::output_state(&output);
            os.momentum.stop();
            os.edge_pan_velocity = None;
            os.panning = false;
            os.camera_target = None;
            os.zoom_target = None;
            os.zoom_animation_center = None;
        }
        self.held_action = None;
        self.cursor.grab_cursor = false;
        if let Some(pending) = self.gestures.pending_middle_click.take() {
            self.loop_handle.remove(pending.timer_token);
        }
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        let pointer = self.pointer();
        pointer.unset_grab(self, serial, 0);

        self.cursor.exec_cursor_show_at = None;
        self.cursor.exec_cursor_deadline = None;
        self.cursor.cursor_status = smithay::input::pointer::CursorImageStatus::default_named();
        // Clear keyboard focus — no window should be interactable
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        let keyboard = self.keyboard();
        keyboard.set_focus(self, None::<FocusTarget>, serial);
        self.mark_all_dirty();
    }

    fn unlock(&mut self) {
        tracing::info!("Session unlocked");
        self.session_lock = SessionLock::Unlocked;
        self.lock_surfaces.clear();
        // Restore focus to the most recent window
        if let Some(window) = self.focus_history.first().cloned() {
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            let keyboard = self.keyboard();
            let focus = window.wl_surface().map(|s| FocusTarget(s.into_owned()));
            keyboard.set_focus(self, focus, serial);
        }
        self.mark_all_dirty();
    }

    fn new_surface(&mut self, surface: LockSurface, wl_output: WlOutput) {
        let output =
            smithay::output::Output::from_resource(&wl_output).or_else(|| self.active_output());
        let Some(output) = output else { return };

        let output_size = crate::state::output_logical_size(&output);

        surface.with_pending_state(|state| {
            state.size = Some((output_size.w as u32, output_size.h as u32).into());
        });
        surface.send_configure();
        self.lock_surfaces.insert(output, surface);
    }
}

delegate_session_lock!(Srwc);
