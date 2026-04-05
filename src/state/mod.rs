mod animation;
use std::path::PathBuf;
mod cursor;
pub mod fit;
mod fullscreen;
mod navigation;
mod render_cache;
pub use cursor::{CursorFrames, CursorState};
pub use render_cache::RenderCache;

use smithay::{
    desktop::{PopupManager, Space, Window},
    input::{Seat, SeatState, keyboard::XkbConfig},
    output::Output,
    reexports::{
        calloop::{LoopHandle, LoopSignal},
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            DisplayHandle, Resource,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Logical, Point, Rectangle, Size},
    wayland::input_method::InputMethodManagerState,
    wayland::output::OutputManagerState,
    wayland::text_input::TextInputManagerState,
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        cursor_shape::CursorShapeManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::XdgShellState,
        shm::ShmState,
    },
};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::GlesTexture;
use smithay::utils::Physical;
use smithay::wayland::content_type::ContentTypeState;
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufState};
use smithay::wayland::fractional_scale::FractionalScaleManagerState;
use smithay::wayland::idle_inhibit::IdleInhibitManagerState;
use smithay::wayland::idle_notify::IdleNotifierState;
use smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState;
use smithay::wayland::pointer_constraints::PointerConstraintsState;
use smithay::wayland::pointer_gestures::PointerGesturesState;
use smithay::wayland::presentation::PresentationState;
use smithay::wayland::relative_pointer::RelativePointerManagerState;
use smithay::wayland::selection::primary_selection::PrimarySelectionState;
use smithay::wayland::selection::wlr_data_control::DataControlState;
use smithay::wayland::session_lock::{LockSurface, SessionLockManagerState, SessionLocker};
use smithay::wayland::shell::wlr_layer::WlrLayerShellState;
use smithay::wayland::shell::xdg::decoration::XdgDecorationState;
use smithay::wayland::single_pixel_buffer::SinglePixelBufferState;
use smithay::wayland::viewporter::ViewporterState;
use smithay::wayland::xdg_activation::XdgActivationState;
use smithay::wayland::xdg_foreign::XdgForeignState;

use smithay::backend::session::libseat::LibSeatSession;
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::xwayland_shell::XWaylandShellState;
use smithay::xwayland::X11Surface;
use smithay::xwayland::xwm::X11Wm;

use smithay::reexports::calloop::RegistrationToken;
use smithay::reexports::drm::control::crtc;

use crate::backend::Backend;
use crate::input::gestures::GestureState;
use crate::screenshot_ui::ScreenshotUi;
use srwm::canvas::MomentumState;
use srwm::config::Config;
use srwm::window_ext::WindowExt;

/// A layer surface placed at a fixed canvas position (instead of screen-anchored via LayerMap).
/// Created when a layer surface's namespace matches a window rule with `position`.
pub struct CanvasLayer {
    pub surface: smithay::desktop::LayerSurface,
    /// Rule position (Y-up, window-centered) — converted to canvas coords after first commit.
    pub rule_position: (i32, i32),
    /// Internal canvas position (Y-down, top-left). None until first commit reveals size.
    pub position: Option<Point<i32, Logical>>,
    pub namespace: String,
}

/// Persistent per-output state for screen recording capture, reused across frames
/// so the damage tracker's age increments and smithay only re-renders damaged regions.
pub struct CaptureOutputState {
    pub damage_tracker: OutputDamageTracker,
    /// Reused offscreen texture for SHM captures (avoids allocation per frame).
    pub offscreen_texture: Option<(GlesTexture, Size<i32, Physical>)>,
    pub age: usize,
    /// Reset age when cursor inclusion changes between frames.
    pub last_paint_cursors: bool,
}

/// Buffered middle-click from a 3-finger tap. Held for DOUBLE_TAP_WINDOW_MS
/// to see if a 3-finger swipe follows (→ move window). If the timer fires
/// without a swipe, the click is forwarded to the client (paste).
pub struct PendingMiddleClick {
    pub press_time: u32,
    pub release_time: Option<u32>,
    pub timer_token: RegistrationToken,
}

/// Session lock state machine: Unlocked → Pending → Locked → Unlocked.
pub enum SessionLock {
    Unlocked,
    /// Lock requested; screen goes black until lock surface commits.
    Pending(SessionLocker),
    /// Lock confirmed; rendering only the lock surface.
    Locked,
}

pub use crate::focus::FocusTarget;

/// Log an error result with context, discarding the Ok value.
#[inline]
pub(crate) fn log_err(context: &str, result: Result<impl Sized, impl std::fmt::Display>) {
    if let Err(e) = result {
        tracing::error!("{context}: {e}");
    }
}

/// Spawn a shell command with SIGCHLD reset to default.
/// The compositor sets SIG_IGN on SIGCHLD for zombie reaping, but children
/// inherit this — breaking GLib's waitpid()-based subprocess management
/// (swaync-client hangs because GSpawnSync gets ECHILD).
pub fn spawn_command(cmd: &str) {
    use std::os::unix::process::CommandExt;
    let mut child = std::process::Command::new("sh");
    child.args(["-c", cmd]);
    unsafe {
        child.pre_exec(|| {
            libc::signal(libc::SIGCHLD, libc::SIG_DFL);
            Ok(())
        });
    }
    log_err("spawn command", child.spawn());
}

/// Saved viewport state for HomeToggle return — includes optional fullscreen window.
#[derive(Clone)]
pub struct HomeReturn {
    pub camera: Point<f64, Logical>,
    pub zoom: f64,
    pub fullscreen_window: Option<Window>,
}

/// Saved state for a fullscreen window — restored on exit.
pub struct FullscreenState {
    pub window: Window,
    pub saved_location: Point<i32, Logical>,
    pub saved_camera: Point<f64, Logical>,
    pub saved_zoom: f64,
    pub saved_size: Size<i32, Logical>,
}

/// Per-output viewport state, stored on each `Output` via `UserDataMap`.
/// Wrapped in `Mutex` since `UserDataMap` requires `Sync`.
/// Fields that are !Send (PixelShaderElement) stay on Srwm.
/// Fields with non-Copy ownership types (fullscreen, lock_surface)
/// stay on Srwm for Phase 1 — moved here when multi-output needs them.
#[derive(Clone)]
pub struct OutputState {
    pub camera: Point<f64, Logical>,
    pub zoom: f64,
    pub zoom_target: Option<f64>,
    pub zoom_animation_center: Option<Point<f64, Logical>>,
    pub last_rendered_zoom: f64,
    pub overview_return: Option<(Point<f64, Logical>, f64)>,
    pub camera_target: Option<Point<f64, Logical>>,
    pub last_scroll_pan: Option<Instant>,
    pub momentum: MomentumState,
    pub panning: bool,
    pub edge_pan_velocity: Option<Point<f64, Logical>>,
    pub last_rendered_camera: Point<f64, Logical>,
    pub last_frame_instant: Instant,
    /// Physical arrangement position in layout space.
    /// (0,0) for single output; from config for multi-monitor.
    pub layout_position: Point<i32, Logical>,
    /// Saved home position for HomeToggle (per-output).
    pub home_return: Option<HomeReturn>,
}

/// Initialize per-output state on a newly created output.
pub fn init_output_state(
    output: &Output,
    camera: Point<f64, Logical>,
    friction: f64,
    layout_position: Point<i32, Logical>,
) {
    if output.user_data().get::<Mutex<OutputState>>().is_some() {
        tracing::warn!("OutputState already initialized for output, skipping");
        return;
    }
    output.user_data().insert_if_missing_threadsafe(|| {
        Mutex::new(OutputState {
            camera,
            zoom: 1.0,
            zoom_target: None,
            zoom_animation_center: None,
            last_rendered_zoom: f64::NAN,
            overview_return: None,
            camera_target: None,
            last_scroll_pan: None,
            momentum: MomentumState::new(friction),
            panning: false,
            edge_pan_velocity: None,
            last_rendered_camera: Point::from((f64::NAN, f64::NAN)),
            last_frame_instant: Instant::now(),
            layout_position,
            home_return: None,
        })
    });
}

/// Screen-space center of an output's usable area (for per-output animation paths).
pub fn usable_center_for_output(output: &Output) -> Point<f64, Logical> {
    let map = smithay::desktop::layer_map_for_output(output);
    let zone = map.non_exclusive_zone();
    Point::from((
        zone.loc.x as f64 + zone.size.w as f64 / 2.0,
        zone.loc.y as f64 + zone.size.h as f64 / 2.0,
    ))
}

/// Logical output size accounting for scale and transform (90°/270° swap width/height).
pub fn output_logical_size(output: &Output) -> Size<i32, Logical> {
    let scale = output.current_scale().fractional_scale();
    output
        .current_mode()
        .map(|m| {
            output
                .current_transform()
                .transform_size(m.size)
                .to_f64()
                .to_logical(scale)
                .to_i32_ceil()
        })
        .unwrap_or((1, 1).into())
}

/// Get a lock on an output's per-output state.
pub fn output_state(output: &Output) -> MutexGuard<'_, OutputState> {
    output
        .user_data()
        .get::<Mutex<OutputState>>()
        .expect("OutputState not initialized on output")
        .lock()
        .expect("OutputState mutex poisoned")
}

fn state_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state"))
        })?;
    Some(base.join("srwm"))
}

impl Srwm {
    pub fn save_cameras(&self) {
        let Some(dir) = state_dir() else { return };
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }

        // Build a TOML string: [output-name]\ncamera_x = ...\ncamera_y = ...\nzoom = ...\n
        let mut content = String::new();
        for output in self.space.outputs() {
            let name = output.name();
            let os = output_state(output);
            content += &format!(
                "[\"{}\"]\ncamera_x = {:.1}\ncamera_y = {:.1}\nzoom = {:.3}\n\n",
                name, os.camera.x, os.camera.y, os.zoom
            );
        }

        let tmp = dir.join("cameras.toml.tmp");
        let path = dir.join("cameras.toml");
        if std::fs::write(&tmp, &content).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

pub fn load_cameras() -> HashMap<String, (Point<f64, Logical>, f64)> {
    load_cameras_from(state_dir())
}

pub(crate) fn load_cameras_from(
    dir: Option<PathBuf>,
) -> HashMap<String, (Point<f64, Logical>, f64)> {
    let mut result = HashMap::new();
    let Some(dir) = dir else {
        return result;
    };
    let Ok(content) = std::fs::read_to_string(dir.join("cameras.toml")) else {
        return result;
    };
    let Ok(table) = content.parse::<toml::Table>() else {
        return result;
    };

    for (name, value) in &table {
        let Some(section) = value.as_table() else {
            continue;
        };
        let cx = section.get("camera_x").and_then(|v| v.as_float());
        let cy = section.get("camera_y").and_then(|v| v.as_float());
        let z = section.get("zoom").and_then(|v| v.as_float());
        if let (Some(x), Some(y), Some(zoom)) = (cx, cy, z) {
            result.insert(name.clone(), (Point::from((x, y)), zoom));
        }
    }
    result
}

pub struct XWaylandContext {
    pub shell_state: XWaylandShellState,
    pub wm: Option<X11Wm>,
    /// Override-redirect X11 windows (menus, tooltips) — rendered manually, not in Space.
    pub override_redirect: Vec<X11Surface>,
    pub display: Option<u32>,
    /// XWayland client handle, stored for reconnect/cleanup.
    pub client: Option<smithay::reexports::wayland_server::Client>,
}

pub struct GestureContext {
    pub state: Option<GestureState>,
    /// The output a gesture started on (pinned for duration of gesture).
    pub pinned_output: Option<Output>,
    /// Fullscreen window that was exited by a gesture (saved before execute_action sees it).
    pub exited_fullscreen: Option<Window>,
    pub pending_middle_click: Option<PendingMiddleClick>,
}

pub struct DrmState {
    pub active_crtcs: HashSet<crtc::Handle>,
    pub redraws_needed: HashSet<crtc::Handle>,
    pub frames_pending: HashSet<crtc::Handle>,
}

pub struct SessionContext {
    pub session: Option<LibSeatSession>,
    pub input_devices: Vec<smithay::reexports::input::Device>,
}

/// Central compositor state.
pub struct Srwm {
    // -- global: infrastructure --
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_handle: LoopHandle<'static, Srwm>,
    pub loop_signal: LoopSignal,

    // -- global: desktop --
    pub space: Space<Window>,
    pub popups: PopupManager,

    // -- global: protocol state --
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    #[allow(dead_code)]
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Srwm>,
    pub data_device_state: DataDeviceState,

    // -- global: input --
    pub seat: Seat<Srwm>,

    // -- global: cursor --
    pub cursor: CursorState,

    // -- global: backend --
    pub backend: Option<Backend>,
    // -- global: SSD decorations --
    pub decorations: HashMap<
        smithay::reexports::wayland_server::backend::ObjectId,
        crate::decorations::WindowDecoration,
    >,
    pub pending_ssd: HashSet<smithay::reexports::wayland_server::backend::ObjectId>,
    // -- global: render state (shaders, blur, backgrounds, captures) --
    pub render: RenderCache,

    // -- global: protocol state (held for smithay delegate macros) --
    pub dmabuf_state: DmabufState,
    pub dmabuf_global: Option<DmabufGlobal>,
    #[allow(dead_code)]
    pub cursor_shape_state: CursorShapeManagerState,
    #[allow(dead_code)]
    pub viewporter_state: ViewporterState,
    #[allow(dead_code)]
    pub fractional_scale_state: FractionalScaleManagerState,
    pub xdg_activation_state: XdgActivationState,
    pub primary_selection_state: PrimarySelectionState,
    pub data_control_state: DataControlState,
    #[allow(dead_code)]
    pub pointer_constraints_state: PointerConstraintsState,
    #[allow(dead_code)]
    pub relative_pointer_state: RelativePointerManagerState,
    #[allow(dead_code)]
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    #[allow(dead_code)]
    pub idle_inhibit_state: IdleInhibitManagerState,
    pub idle_notifier_state: IdleNotifierState<Srwm>,
    #[allow(dead_code)]
    pub presentation_state: PresentationState,
    #[allow(dead_code)]
    pub decoration_state: XdgDecorationState,
    pub layer_shell_state: WlrLayerShellState,
    pub foreign_toplevel_state: srwm::protocols::foreign_toplevel::ForeignToplevelManagerState,
    pub screencopy_state: srwm::protocols::screencopy::ScreencopyManagerState,
    pub output_management_state: srwm::protocols::output_management::OutputManagementState,
    pub pending_screencopies: Vec<srwm::protocols::screencopy::Screencopy>,
    #[allow(dead_code)]
    pub image_capture_source_state: srwm::protocols::image_capture_source::ImageCaptureSourceState,
    pub image_copy_capture_state: srwm::protocols::image_copy_capture::ImageCopyCaptureState,
    pub pending_captures: Vec<srwm::protocols::image_copy_capture::PendingCapture>,
    pub xdg_foreign_state: XdgForeignState,
    pub session_lock_manager_state: SessionLockManagerState,
    pub session_lock: SessionLock,
    // -- per-output: lock surface (one per output in multi-monitor) --
    pub lock_surfaces: HashMap<Output, LockSurface>,

    // -- global: pointer/layer state --
    pub pointer_over_layer: bool,
    pub canvas_layers: Vec<CanvasLayer>,

    // -- global: config --
    pub config: Config,

    // -- global: window management --
    pub pending_center: HashSet<WlSurface>,
    pub pending_size: HashSet<WlSurface>,

    // -- global: focus/navigation --
    pub focus_history: Vec<Window>,
    pub cycle_state: Option<usize>,

    // -- global: key repeat --
    pub held_action: Option<(u32, srwm::config::Action, Instant)>,

    // -- per-output: fullscreen (keyed by output, since FullscreenState has Window) --
    pub fullscreen: HashMap<Output, FullscreenState>,

    // -- global: gesture state --
    pub gestures: GestureContext,

    // -- global: momentum launch timer --
    pub momentum_timer: Option<RegistrationToken>,

    // -- global: session --
    pub session_ctx: SessionContext,
    pub active_layout: String,

    // -- global: autostart --
    pub autostart: Vec<String>,

    // -- global: udev/DRM --
    pub drm: DrmState,

    // -- global: config hot-reload --
    pub config_file_mtime: Option<std::time::SystemTime>,

    // -- global: multi-monitor --
    /// Global animation tick timestamp — used for dt computation in tick_all_animations().
    /// Separate from per-output last_frame_instant to avoid double-ticking when multiple
    /// outputs render in one iteration.
    pub last_animation_tick: Instant,
    /// The output the pointer is currently on (for input routing).
    pub focused_output: Option<Output>,
    /// Output names kept as virtual placeholders when all physical outputs disconnect.
    /// Prevents `active_output().unwrap()` panics by keeping the output in the Space.
    pub disconnected_outputs: HashSet<String>,
    /// Set when output config was applied via wlr-output-management; render loop
    /// should re-collect output state and notify clients.
    pub output_config_dirty: bool,

    // -- global: XWayland --
    pub xwayland: XWaylandContext,

    // -- global: SSD title bar double-click --
    // -- global: screenshot UI --
    pub screenshot_ui: ScreenshotUi,
    pub pending_screenshot: bool,
    pub pending_screenshot_screen: bool,
    pub pending_screenshot_confirm: Option<bool>,

    pub last_titlebar_click: Option<(
        Instant,
        smithay::reexports::wayland_server::backend::ObjectId,
    )>,

    // -- global: screencasting --
    pub screencasting: Option<crate::screencasting::Screencasting>,
    pub conn_screen_cast: Option<zbus::blocking::Connection>,
    pub gbm_device:
        Option<smithay::backend::allocator::gbm::GbmDevice<smithay::backend::drm::DrmDeviceFd>>,
}

/// Per-client state stored by wayland-server for each connected client.
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

impl Srwm {
    pub fn new(
        dh: DisplayHandle,
        loop_handle: LoopHandle<'static, Srwm>,
        loop_signal: LoopSignal,
    ) -> Self {
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new_with_capabilities::<Self>(
            &dh,
            [
                xdg_toplevel::WmCapabilities::Fullscreen,
                xdg_toplevel::WmCapabilities::Maximize,
            ],
        );
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&dh);

        let cursor_shape_state = CursorShapeManagerState::new::<Self>(&dh);
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let fractional_scale_state = FractionalScaleManagerState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        SinglePixelBufferState::new::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh);
        let data_control_state =
            DataControlState::new::<Self, _>(&dh, Some(&primary_selection_state), |_| true);
        let pointer_constraints_state = PointerConstraintsState::new::<Self>(&dh);
        let relative_pointer_state = RelativePointerManagerState::new::<Self>(&dh);
        let _pointer_gestures_state = PointerGesturesState::new::<Self>(&dh);
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);
        let idle_inhibit_state = IdleInhibitManagerState::new::<Self>(&dh);
        let idle_notifier_state = IdleNotifierState::new(&dh, loop_handle.clone());
        let presentation_state = PresentationState::new::<Self>(&dh, 1); // CLOCK_MONOTONIC
        let decoration_state = XdgDecorationState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let foreign_toplevel_state =
            srwm::protocols::foreign_toplevel::ForeignToplevelManagerState::new::<Self, _>(
                &dh,
                |_| true,
            );
        let screencopy_state =
            srwm::protocols::screencopy::ScreencopyManagerState::new::<Self, _>(&dh, |_| true);
        let image_capture_source_state =
            srwm::protocols::image_capture_source::ImageCaptureSourceState::new::<Self, _>(
                &dh,
                |_| true,
            );
        let image_copy_capture_state =
            srwm::protocols::image_copy_capture::ImageCopyCaptureState::new::<Self, _>(&dh, |_| {
                true
            });
        let output_management_state =
            srwm::protocols::output_management::OutputManagementState::new::<Self, _>(&dh, |_| {
                true
            });
        let session_lock_manager_state = SessionLockManagerState::new::<Self, _>(&dh, |_| true);
        let xwayland_shell_state = XWaylandShellState::new::<Self>(&dh);
        let xdg_foreign_state = XdgForeignState::new::<Self>(&dh);
        ContentTypeState::new::<Self>(&dh);
        {
            use smithay::wayland::shell::xdg::dialog::XdgDialogState;
            XdgDialogState::new::<Self>(&dh);
        }
        {
            use smithay::wayland::xwayland_keyboard_grab::XWaylandKeyboardGrabState;
            XWaylandKeyboardGrabState::new::<Self>(&dh);
        }
        TextInputManagerState::new::<Self>(&dh);
        InputMethodManagerState::new::<Self, _>(&dh, |_client| true);
        smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState::new::<Self, _>(
            &dh,
            |_client| true,
        );

        let config = Config::load();

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "seat-0");
        let kb = &config.input.keyboard_layout;
        let xkb = XkbConfig {
            layout: &kb.layout,
            variant: &kb.variant,
            options: if kb.options.is_empty() {
                None
            } else {
                Some(kb.options.clone())
            },
            model: &kb.model,
            ..Default::default()
        };
        seat.add_keyboard(xkb, config.input.repeat_delay, config.input.repeat_rate)
            .expect("Failed to add keyboard");
        seat.add_pointer();
        let autostart = config.autostart.clone();
        Self {
            start_time: Instant::now(),
            display_handle: dh,
            loop_handle,
            loop_signal,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
            cursor: CursorState::new(),
            backend: None,
            decorations: HashMap::new(),
            pending_ssd: HashSet::new(),
            render: RenderCache::new(),
            dmabuf_state: DmabufState::new(),
            dmabuf_global: None,
            cursor_shape_state,
            viewporter_state,
            fractional_scale_state,
            xdg_activation_state,
            primary_selection_state,
            data_control_state,
            pointer_constraints_state,
            relative_pointer_state,
            keyboard_shortcuts_inhibit_state,
            idle_inhibit_state,
            idle_notifier_state,
            presentation_state,
            decoration_state,
            layer_shell_state,
            foreign_toplevel_state,
            screencopy_state,
            output_management_state,
            pending_screencopies: Vec::new(),
            image_capture_source_state,
            image_copy_capture_state,
            pending_captures: Vec::new(),
            xdg_foreign_state,
            session_lock_manager_state,
            session_lock: SessionLock::Unlocked,
            lock_surfaces: HashMap::new(),
            pointer_over_layer: false,
            canvas_layers: Vec::new(),
            config,
            pending_center: HashSet::new(),
            pending_size: HashSet::new(),
            focus_history: Vec::new(),
            cycle_state: None,
            held_action: None,
            gestures: GestureContext {
                state: None,
                pinned_output: None,
                exited_fullscreen: None,
                pending_middle_click: None,
            },
            momentum_timer: None,
            fullscreen: HashMap::new(),
            session_ctx: SessionContext {
                session: None,
                input_devices: Vec::new(),
            },
            active_layout: String::new(),
            autostart,
            drm: DrmState {
                active_crtcs: HashSet::new(),
                redraws_needed: HashSet::new(),
                frames_pending: HashSet::new(),
            },
            config_file_mtime: None,
            last_animation_tick: Instant::now(),
            focused_output: None,
            disconnected_outputs: HashSet::new(),
            output_config_dirty: false,
            xwayland: XWaylandContext {
                shell_state: xwayland_shell_state,
                wm: None,
                override_redirect: Vec::new(),
                display: None,
                client: None,
            },
            screenshot_ui: ScreenshotUi::new(),
            pending_screenshot: false,
            pending_screenshot_screen: false,
            pending_screenshot_confirm: None,
            last_titlebar_click: None,
            screencasting: None,
            conn_screen_cast: None,
            gbm_device: None,
        }
    }

    /// Push any `below` windows to the bottom of the z-order.
    /// Called after every `raise_element()` to maintain stacking.
    pub fn enforce_below_windows(&mut self) {
        self.render.blur_scene_generation += 1;
        self.render.blur_geometry_generation += 1;
        // Space stores elements in a vec where last = topmost.
        // raise_element pushes to the end (top). So we raise all
        // non-below windows in reverse order to preserve their relative
        // stacking while ensuring they sit above any below windows.
        let non_below: Vec<_> = self
            .space
            .elements()
            .filter(|w| {
                !w.wl_surface()
                    .and_then(|s| srwm::config::applied_rule(&s))
                    .is_some_and(|r| r.widget)
            })
            .cloned()
            .collect();

        for w in non_below {
            self.space.raise_element(&w, false);
        }

        // Parent-child stacking: raise children after their parents so
        // they always appear on top. Works naturally for nested hierarchies.
        let parented: Vec<Window> = self
            .space
            .elements()
            .filter(|w| w.parent_surface().is_some())
            .cloned()
            .collect();
        for child in parented {
            self.space.raise_element(&child, false);
        }

        for fs in self.fullscreen.values() {
            self.space.raise_element(&fs.window, false);
        }
    }

    /// Find the Window in space whose wl_surface matches the given one.
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|w| w.wl_surface().as_deref() == Some(surface))
            .cloned()
    }

    /// Get the innermost modal child of a window (for focus redirect).
    /// Recursively chases modal chains (e.g. file picker → overwrite confirm).
    /// Capped at 10 iterations to guard against circular parents.
    pub fn topmost_modal_child(&self, window: &Window) -> Option<Window> {
        let parent_surface = window.wl_surface()?;
        let child = self
            .space
            .elements()
            .rfind(|w| w.parent_surface().as_ref() == Some(&*parent_surface) && w.is_modal())
            .cloned()?;
        self.topmost_modal_child_inner(&child, 9).or(Some(child))
    }

    fn topmost_modal_child_inner(&self, window: &Window, depth: u8) -> Option<Window> {
        if depth == 0 {
            return None;
        }
        let parent_surface = window.wl_surface()?;
        let child = self
            .space
            .elements()
            .rfind(|w| w.parent_surface().as_ref() == Some(&*parent_surface) && w.is_modal())
            .cloned()?;
        self.topmost_modal_child_inner(&child, depth - 1)
            .or(Some(child))
    }

    /// Raise a window and set keyboard focus, with modal focus redirect.
    /// If the window has a modal child, focus goes to that child instead.
    pub fn raise_and_focus(&mut self, window: &Window, serial: smithay::utils::Serial) {
        self.space.raise_element(window, true);
        self.enforce_below_windows();

        // Resolve focus target before borrowing keyboard (modal redirect)
        let focus_surface = self
            .topmost_modal_child(window)
            .or(Some(window.clone()))
            .and_then(|w| w.wl_surface().map(|s| FocusTarget(s.into_owned())));

        let keyboard = self.keyboard();
        keyboard.set_focus(self, focus_surface, serial);
    }

    /// Find a mapped window wrapping the given X11 surface.
    pub fn find_x11_window(&self, x11: &X11Surface) -> Option<Window> {
        self.space
            .elements()
            .find(|w| w.x11_surface() == Some(x11))
            .cloned()
    }

    /// Find the X11Surface whose underlying wl_surface matches the given one.
    pub fn find_x11_surface_by_wl(&self, wl: &WlSurface) -> Option<X11Surface> {
        self.space
            .elements()
            .filter_map(|w| w.x11_surface().cloned())
            .find(|x11| x11.wl_surface().as_ref() == Some(wl))
    }

    /// Compute the canvas position of an override-redirect X11 surface.
    /// OR windows use absolute X11 root coords; we map them relative to
    /// their parent's canvas position, or center them if no parent exists.
    pub fn or_canvas_position(&self, or_surface: &X11Surface) -> Point<i32, Logical> {
        let or_geo = or_surface.geometry();

        if let Some(parent_id) = or_surface.is_transient_for() {
            // Search managed windows in Space for parent
            let parent_in_space = self
                .space
                .elements()
                .find(|w| w.x11_surface().is_some_and(|x| x.window_id() == parent_id));
            if let Some(parent_win) = parent_in_space {
                let parent_canvas = self.space.element_location(parent_win).unwrap_or_default();
                let parent_x11_loc = parent_win.x11_surface().unwrap().geometry().loc;
                return parent_canvas + (or_geo.loc - parent_x11_loc);
            }

            // Search other OR windows (nested menus) with depth limit
            fn find_or_parent(
                or_list: &[X11Surface],
                space: &smithay::desktop::Space<smithay::desktop::Window>,
                target_id: u32,
                depth: u32,
            ) -> Option<Point<i32, Logical>> {
                if depth == 0 {
                    return None;
                }
                let parent_or = or_list.iter().find(|w| w.window_id() == target_id)?;
                let parent_geo = parent_or.geometry();
                if let Some(grandparent_id) = parent_or.is_transient_for() {
                    // Check Space first
                    let gp_in_space = space.elements().find(|w| {
                        w.x11_surface()
                            .is_some_and(|x| x.window_id() == grandparent_id)
                    });
                    if let Some(gp_win) = gp_in_space {
                        let gp_canvas = space.element_location(gp_win).unwrap_or_default();
                        let gp_x11_loc = gp_win.x11_surface().unwrap().geometry().loc;
                        return Some(gp_canvas + (parent_geo.loc - gp_x11_loc));
                    }
                    // Recurse into OR list
                    let gp_canvas = find_or_parent(or_list, space, grandparent_id, depth - 1)?;
                    return Some(
                        gp_canvas
                            + (parent_geo.loc
                                - or_list
                                    .iter()
                                    .find(|w| w.window_id() == grandparent_id)
                                    .map(|w| w.geometry().loc)
                                    .unwrap_or_default()),
                    );
                }
                None
            }

            if let Some(parent_canvas) =
                find_or_parent(&self.xwayland.override_redirect, &self.space, parent_id, 10)
            {
                let parent_or = self
                    .xwayland
                    .override_redirect
                    .iter()
                    .find(|w| w.window_id() == parent_id);
                let parent_x11_loc = parent_or.map(|w| w.geometry().loc).unwrap_or_default();
                return parent_canvas + (or_geo.loc - parent_x11_loc);
            }
        }

        // No transient_for: use anchor-based X11→canvas coordinate mapping.
        // X11 OR windows position themselves in absolute root coords — find
        // the topmost managed X11 window as an anchor to translate.
        let anchor = self.space.elements().rev().find_map(|w| {
            let x11 = w.x11_surface()?;
            let canvas_loc = self.space.element_location(w)?;
            Some((canvas_loc, x11.geometry().loc))
        });
        if let Some((anchor_canvas, anchor_x11)) = anchor {
            return anchor_canvas + (or_geo.loc - anchor_x11);
        }

        // No X11 windows at all: center in viewport
        self.active_output()
            .and_then(|o| self.space.output_geometry(&o))
            .map(|viewport| {
                let (cam, z) = self
                    .with_output_state(|os| (os.camera, os.zoom))
                    .unwrap_or_default();
                Point::from((
                    (cam.x + viewport.size.w as f64 / (2.0 * z)) as i32 - or_geo.size.w / 2,
                    (cam.y + viewport.size.h as f64 / (2.0 * z)) as i32 - or_geo.size.h / 2,
                ))
            })
            .unwrap_or_default()
    }

    /// Mark all active outputs as needing a redraw.
    pub fn mark_all_dirty(&mut self) {
        self.drm.redraws_needed.extend(self.drm.active_crtcs.iter());
    }

    pub fn remove_capture_state(&mut self, output_name: &str) {
        self.render.remove_capture_state(output_name);
    }

    pub fn cursor_is_animated(&self) -> bool {
        self.cursor.is_animated()
    }

    /// True if a specific output has per-output animations in progress.
    pub fn output_has_active_animations(&self, output: &Output) -> bool {
        let os = output_state(output);
        os.camera_target.is_some()
            || os.zoom_target.is_some()
            || os.edge_pan_velocity.is_some()
            || os.momentum.velocity.x != 0.0
            || os.momentum.velocity.y != 0.0
    }

    /// True if any animation is still in progress and needs continued rendering.
    pub fn has_active_animations(&self) -> bool {
        self.space
            .outputs()
            .any(|o| self.output_has_active_animations(o))
            || self.held_action.is_some()
            || self.cursor.exec_cursor_show_at.is_some()
            || self.cursor.exec_cursor_deadline.is_some()
            || self.cursor.is_animated()
    }

    /// Forward a buffered middle-click press+release to the client.
    pub fn flush_middle_click(&mut self, press_time: u32, release_time: Option<u32>) {
        let pointer = self.pointer();
        let serial = smithay::utils::SERIAL_COUNTER.next_serial();
        pointer.button(
            self,
            &smithay::input::pointer::ButtonEvent {
                button: srwm::config::BTN_MIDDLE,
                state: smithay::backend::input::ButtonState::Pressed,
                serial,
                time: press_time,
            },
        );
        pointer.frame(self);
        if let Some(rt) = release_time {
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            pointer.button(
                self,
                &smithay::input::pointer::ButtonEvent {
                    button: srwm::config::BTN_MIDDLE,
                    state: smithay::backend::input::ButtonState::Released,
                    serial,
                    time: rt,
                },
            );
            pointer.frame(self);
        }
    }

    /// Flush the pending middle-click (called by calloop timer when no swipe followed).
    pub fn flush_pending_middle_click(&mut self) {
        let Some(pending) = self.gestures.pending_middle_click.take() else {
            return;
        };
        self.flush_middle_click(pending.press_time, pending.release_time);
    }

    /// The output the pointer is currently on.
    /// Returns `focused_output` with fallback to first output.
    pub fn active_output(&self) -> Option<Output> {
        self.focused_output
            .clone()
            .or_else(|| self.space.outputs().next().cloned())
    }

    /// Get the fullscreen state for the active output (if any).
    pub fn active_fullscreen(&self) -> Option<&FullscreenState> {
        self.active_output().and_then(|o| self.fullscreen.get(&o))
    }

    /// Check if the active output is in fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        self.active_output()
            .is_some_and(|o| self.fullscreen.contains_key(&o))
    }

    /// Check if a specific output is in fullscreen mode.
    pub fn is_output_fullscreen(&self, output: &Output) -> bool {
        self.fullscreen.contains_key(output)
    }

    /// Find the output whose viewport contains (or is nearest to) a window's center.
    /// Falls back to active output if the window isn't visible on any output.
    pub fn output_for_window(&self, window: &smithay::desktop::Window) -> Option<Output> {
        let loc = self.space.element_location(window)?;
        let geo = window.geometry();
        let center: Point<f64, Logical> = Point::from((
            loc.x as f64 + geo.size.w as f64 / 2.0,
            loc.y as f64 + geo.size.h as f64 / 2.0,
        ));
        // Find which output's visible canvas rect contains the window center.
        let found = self
            .space
            .outputs()
            .find(|output| {
                let os = output_state(output);
                let size = output_logical_size(output);
                let visible =
                    srwm::canvas::visible_canvas_rect(os.camera.to_i32_round(), size, os.zoom);
                drop(os);
                visible.contains(Point::from((center.x as i32, center.y as i32)))
            })
            .cloned();
        found.or_else(|| self.active_output())
    }

    /// Find the nearest output in the given direction from `from`.
    pub fn output_in_direction(
        &self,
        from: &Output,
        dir: &srwm::config::Direction,
    ) -> Option<Output> {
        let from_center: Point<f64, Logical> = {
            let os = output_state(from);
            let size = output_logical_size(from);
            Point::from((
                os.layout_position.x as f64 + size.w as f64 / 2.0,
                os.layout_position.y as f64 + size.h as f64 / 2.0,
            ))
        };
        let (dx, dy) = dir.to_unit_vec();

        self.space
            .outputs()
            .filter(|o| *o != from)
            .filter_map(|o| {
                let os = output_state(o);
                let size = output_logical_size(o);
                let center: Point<f64, Logical> = Point::from((
                    os.layout_position.x as f64 + size.w as f64 / 2.0,
                    os.layout_position.y as f64 + size.h as f64 / 2.0,
                ));
                drop(os);
                let to_x = center.x - from_center.x;
                let to_y = center.y - from_center.y;
                let dist = (to_x * to_x + to_y * to_y).sqrt();
                if dist < 1.0 {
                    return None;
                }
                // Check alignment with direction (dot product > 0.5 = within ~60°)
                let dot = (to_x * dx + to_y * dy) / dist;
                if dot > 0.5 {
                    Some((o.clone(), dist))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(o, _)| o)
    }

    /// Find which output's layout rectangle contains `pos` in layout space.
    /// Uses `layout_position` + output mode size (NOT `space.output_geometry()`).
    pub fn output_at_layout_pos(&self, pos: Point<f64, Logical>) -> Option<Output> {
        self.space
            .outputs()
            .find(|output| {
                let os = output_state(output);
                let lp = os.layout_position;
                drop(os);
                let size = output_logical_size(output);
                pos.x >= lp.x as f64
                    && pos.x < (lp.x + size.w) as f64
                    && pos.y >= lp.y as f64
                    && pos.y < (lp.y + size.h) as f64
            })
            .cloned()
    }

    /// Convert canvas position to layout position via an output's camera/zoom.
    /// layout_pos = (canvas - camera) * zoom + layout_position
    #[cfg(test)]
    pub fn canvas_to_layout_pos(
        canvas_pos: Point<f64, Logical>,
        os: &OutputState,
    ) -> Point<f64, Logical> {
        let screen =
            srwm::canvas::canvas_to_screen(srwm::canvas::CanvasPos(canvas_pos), os.camera, os.zoom)
                .0;
        Point::from((
            screen.x + os.layout_position.x as f64,
            screen.y + os.layout_position.y as f64,
        ))
    }

    /// Convert layout position to canvas position via an output's camera/zoom.
    /// canvas = (layout_pos - layout_position) / zoom + camera
    #[cfg(test)]
    pub fn layout_to_canvas_pos(
        layout_pos: Point<f64, Logical>,
        os: &OutputState,
    ) -> Point<f64, Logical> {
        let screen = Point::from((
            layout_pos.x - os.layout_position.x as f64,
            layout_pos.y - os.layout_position.y as f64,
        ));
        srwm::canvas::screen_to_canvas(srwm::canvas::ScreenPos(screen), os.camera, os.zoom).0
    }

    /// Batch-access per-output state for the active output under a single mutex lock.
    pub fn with_output_state<R>(&self, f: impl FnOnce(&mut OutputState) -> R) -> Option<R> {
        let output = self.active_output()?;
        let mut guard = output_state(&output);
        Some(f(&mut guard))
    }

    /// Batch-access per-output state for a specific output under a single mutex lock.
    pub fn with_output_state_on<R>(
        &self,
        output: &Output,
        f: impl FnOnce(&mut OutputState) -> R,
    ) -> R {
        let mut guard = output_state(output);
        f(&mut guard)
    }

    // -- Input helpers --
    pub fn keyboard(&self) -> smithay::input::keyboard::KeyboardHandle<Self> {
        self.seat.get_keyboard().expect("seat has no keyboard")
    }
    pub fn pointer(&self) -> smithay::input::pointer::PointerHandle<Self> {
        self.seat.get_pointer().expect("seat has no pointer")
    }

    // -- Per-output field accessors (delegate to active output's OutputState) --

    /// Sync each output's position to its camera, so render_output
    /// automatically applies the canvas→screen transform.
    pub fn update_output_from_camera(&mut self) {
        let mut changed = false;
        for output in self.space.outputs().cloned().collect::<Vec<_>>() {
            let cam = output_state(&output).camera.to_i32_round();
            if self.space.output_geometry(&output).map(|g| g.loc) != Some(cam) {
                changed = true;
            }
            self.space.map_output(&output, cam);
        }
        if changed {
            self.render.blur_camera_generation += 1;
        }
    }

    /// Logical viewport size of the active (pointer-focused) output.
    pub fn get_viewport_size(&self) -> Size<i32, Logical> {
        self.active_output()
            .map(|o| output_logical_size(&o))
            .unwrap_or((1, 1).into())
    }

    /// Viewport area minus layer-shell exclusive zones (panels, bars).
    pub fn get_usable_area(&self) -> Rectangle<i32, Logical> {
        self.active_output()
            .map(|o| {
                let map = smithay::desktop::layer_map_for_output(&o);
                map.non_exclusive_zone()
            })
            .unwrap_or_else(|| Rectangle::new((0, 0).into(), (1, 1).into()))
    }

    /// Screen-space center of the usable area (accounts for panel exclusive zones).
    /// Without panels, equals (viewport.w/2, viewport.h/2).
    pub fn usable_center_screen(&self) -> Point<f64, Logical> {
        let usable = self.get_usable_area();
        Point::from((
            usable.loc.x as f64 + usable.size.w as f64 / 2.0,
            usable.loc.y as f64 + usable.size.h as f64 / 2.0,
        ))
    }

    /// SSD title bar height for a window (0 for CSD/borderless).
    pub fn window_ssd_bar(&self, window: &Window) -> i32 {
        window
            .wl_surface()
            .filter(|s| self.decorations.contains_key(&s.id()))
            .map_or(0, |_| srwm::config::DecorationConfig::TITLE_BAR_HEIGHT)
    }

    /// Visual center of a window, accounting for SSD title bar above content.
    pub fn window_visual_center(&self, window: &Window) -> Option<Point<f64, Logical>> {
        let loc = self.space.element_location(window)?;
        let size = window.geometry().size;
        let bar = self.window_ssd_bar(window) as f64;
        Some(Point::from((
            loc.x as f64 + size.w as f64 / 2.0,
            loc.y as f64 - bar + (size.h as f64 + bar) / 2.0,
        )))
    }

    /// Offset a spawn position so it doesn't overlap an existing window.
    /// Walks in diagonal steps (title bar height) until no window is within a few pixels.
    pub fn cascade_position(&self, mut pos: (i32, i32), skip: &Window) -> (i32, i32) {
        let step = srwm::config::DecorationConfig::TITLE_BAR_HEIGHT;
        loop {
            let dominated = self.space.elements().any(|w| {
                w != skip
                    && self
                        .space
                        .element_location(w)
                        .is_some_and(|loc| (loc.x - pos.0).abs() <= 2 && (loc.y - pos.1).abs() <= 2)
            });
            if !dominated {
                break pos;
            }
            pos.0 += step;
            pos.1 += step;
        }
    }

    /// Hot-reload config from disk. On parse failure, logs an error and keeps the old config.
    pub fn reload_config(&mut self) {
        let config_path = srwm::config::config_path();
        let contents = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    "Config reload: failed to read {}: {e}",
                    config_path.display()
                );
                return;
            }
        };
        let mut new_config = match srwm::config::Config::from_toml(&contents) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Config reload: parse error: {e}");
                return;
            }
        };

        // Hot-reload keyboard layout
        if new_config.input.keyboard_layout != self.config.input.keyboard_layout {
            let kb = &new_config.input.keyboard_layout;
            let xkb = XkbConfig {
                layout: &kb.layout,
                variant: &kb.variant,
                options: if kb.options.is_empty() {
                    None
                } else {
                    Some(kb.options.clone())
                },
                model: &kb.model,
                ..Default::default()
            };
            let keyboard = self.keyboard();
            let num_lock = keyboard.modifier_state().num_lock;
            if let Err(err) = keyboard.set_xkb_config(self, xkb) {
                tracing::warn!("Config reload: error updating keyboard layout: {err:?}");
                new_config.input.keyboard_layout = self.config.input.keyboard_layout.clone();
            } else {
                tracing::info!("Config reload: keyboard layout updated");
                let mut mods = keyboard.modifier_state();
                if mods.num_lock != num_lock {
                    mods.num_lock = num_lock;
                    keyboard.set_modifier_state(mods);
                }
            }
        }
        if new_config.autostart != self.config.autostart {
            tracing::info!("Config reload: autostart changes only apply at startup");
        }

        // Keyboard repeat rate/delay
        if new_config.input.repeat_rate != self.config.input.repeat_rate
            || new_config.input.repeat_delay != self.config.input.repeat_delay
        {
            let keyboard = self.keyboard();
            keyboard
                .change_repeat_info(new_config.input.repeat_rate, new_config.input.repeat_delay);
        }

        // Momentum friction — apply to all outputs
        if new_config.nav.friction != self.config.nav.friction {
            for output in self.space.outputs() {
                output_state(output).momentum.friction = new_config.nav.friction;
            }
        }

        // Background shader/tile — always clear cached state so that editing
        // the shader file on disk takes effect after `touch`ing the config.
        self.render.background_shader = None;
        self.render.cached_bg_elements.clear();
        self.render.tile_shader = None;
        self.render.cached_tile_bg.clear();

        // Cursor theme/size — validate theme before committing
        let theme_changed = new_config.cursor_theme != self.config.cursor_theme;
        let size_changed = new_config.cursor_size != self.config.cursor_size;
        if theme_changed || size_changed {
            let theme_ok = if theme_changed {
                if let Some(ref theme_name) = new_config.cursor_theme {
                    let theme = xcursor::CursorTheme::load(theme_name);
                    if theme.load_icon("default").is_some() {
                        unsafe { std::env::set_var("XCURSOR_THEME", theme_name) };
                        true
                    } else {
                        tracing::warn!(
                            "Cursor theme '{theme_name}' not found, keeping current theme"
                        );
                        new_config.cursor_theme = self.config.cursor_theme.clone();
                        false
                    }
                } else {
                    unsafe { std::env::remove_var("XCURSOR_THEME") };
                    true
                }
            } else {
                false
            };

            if size_changed {
                if let Some(size) = new_config.cursor_size {
                    unsafe { std::env::set_var("XCURSOR_SIZE", size.to_string()) };
                } else {
                    unsafe { std::env::remove_var("XCURSOR_SIZE") };
                }
            }

            if theme_ok || size_changed {
                self.cursor.cursor_buffers.clear();
            }
        }

        // Trackpad settings — reconfigure all connected devices
        if new_config.input.trackpad != self.config.input.trackpad {
            self.config.input.trackpad = new_config.input.trackpad.clone();
            let devices = self.session_ctx.input_devices.clone();
            for mut device in devices {
                self.configure_libinput_device(&mut device);
            }
            tracing::info!("Config reload: trackpad settings applied to all devices");
        }

        // Env vars — diff old vs new, apply changes
        for (key, value) in &new_config.env {
            if self.config.env.get(key) != Some(value) {
                tracing::info!("Config reload: env {key}={value}");
                unsafe { std::env::set_var(key, value) };
            }
        }
        for key in self.config.env.keys() {
            if !new_config.env.contains_key(key) {
                tracing::info!("Config reload: env unset {key}");
                unsafe { std::env::remove_var(key) };
            }
        }

        self.config = new_config;
        self.mark_all_dirty();
        tracing::info!("Config reloaded");
    }

    pub fn load_xcursor(&mut self, name: &str) -> Option<&CursorFrames> {
        self.cursor.load_xcursor(name)
    }

    pub fn confirm_screenshot(&mut self, write_to_disk: bool) {
        if !self.screenshot_ui.is_open() {
            return;
        }
        self.pending_screenshot_confirm = Some(write_to_disk);
    }

    pub fn save_screenshot(
        &mut self,
        size: Size<i32, Physical>,
        pixels: &[u8],
        write_to_disk: bool,
    ) {
        // Fallback for clipboard if not writing to disk
        if !write_to_disk {
            tracing::info!("Screenshot saved to clipboard via wl-copy");
            use std::io::Write;
            let cmd = std::process::Command::new("wl-copy")
                .arg("-t")
                .arg("image/png")
                .stdin(std::process::Stdio::piped())
                .spawn();
            if let Ok(mut child) = cmd {
                let mut png_data = Vec::new();
                {
                    let mut encoder =
                        png::Encoder::new(&mut png_data, size.w as u32, size.h as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    if let Ok(mut writer) = encoder.write_header() {
                        let _ = writer.write_image_data(pixels);
                    }
                }
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&png_data);
                }
            }
            return;
        }

        // Write to disk
        let Some(dir) = dirs::picture_dir() else {
            tracing::warn!("Could not find picture dir");
            return;
        };
        let screenshots_dir = dir.join("Screenshots");
        let _ = std::fs::create_dir_all(&screenshots_dir);

        let now = chrono::Local::now();
        let filename = format!("screenshot-{}.png", now.format("%Y-%m-%d-%H-%M-%S"));
        let path = screenshots_dir.join(filename);

        if let Ok(file) = std::fs::File::create(&path) {
            let w = &mut std::io::BufWriter::new(file);
            let mut encoder = png::Encoder::new(w, size.w as u32, size.h as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            if let Ok(mut writer) = encoder.write_header()
                && writer.write_image_data(pixels).is_ok()
            {
                tracing::info!("Screenshot saved to {:?}", path);
                // Also fire notify-send
                let _ = std::process::Command::new("notify-send")
                    .args([
                        "-a",
                        "srwm",
                        "-i",
                        "camera",
                        "Screenshot Saved",
                        path.to_str().unwrap(),
                    ])
                    .spawn();
            }
        }
    }
    pub fn restore_pointer_to_canvas(&mut self) {
        let pointer = self.pointer();
        let screen_pos = pointer.current_location();
        let Some((camera, zoom)) = self.with_output_state(|os| (os.camera, os.zoom)) else {
            return;
        };
        let canvas_pos =
            srwm::canvas::screen_to_canvas(srwm::canvas::ScreenPos(screen_pos), camera, zoom).0;
        pointer.motion(
            self,
            None,
            &smithay::input::pointer::MotionEvent {
                location: canvas_pos,
                serial: smithay::utils::SERIAL_COUNTER.next_serial(),
                time: 0,
            },
        );
        pointer.frame(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use srwm::canvas::MomentumState;

    fn mock_output_state(
        camera: (f64, f64),
        zoom: f64,
        layout_position: (i32, i32),
    ) -> OutputState {
        OutputState {
            camera: Point::from(camera),
            zoom,
            zoom_target: None,
            zoom_animation_center: None,
            last_rendered_zoom: zoom,
            overview_return: None,
            camera_target: None,
            last_scroll_pan: None,
            momentum: MomentumState::new(0.96),
            panning: false,
            edge_pan_velocity: None,
            last_rendered_camera: Point::from(camera),
            last_frame_instant: Instant::now(),
            layout_position: Point::from(layout_position),
            home_return: None,
        }
    }

    #[test]
    fn canvas_to_layout_round_trip_zoom_1() {
        let os = mock_output_state((100.0, 200.0), 1.0, (0, 0));
        let canvas = Point::from((150.0, 250.0));
        let layout = Srwm::canvas_to_layout_pos(canvas, &os);
        let back = Srwm::layout_to_canvas_pos(layout, &os);
        assert!((back.x - canvas.x).abs() < 0.001);
        assert!((back.y - canvas.y).abs() < 0.001);
    }

    #[test]
    fn canvas_to_layout_round_trip_with_zoom() {
        let os = mock_output_state((50.0, 75.0), 2.0, (1920, 0));
        let canvas = Point::from((80.0, 100.0));
        let layout = Srwm::canvas_to_layout_pos(canvas, &os);
        let back = Srwm::layout_to_canvas_pos(layout, &os);
        assert!((back.x - canvas.x).abs() < 0.001);
        assert!((back.y - canvas.y).abs() < 0.001);
    }

    #[test]
    fn canvas_to_layout_known_values() {
        // camera=(100,200), zoom=2, layout_position=(1920,0)
        // screen = (canvas - camera) * zoom = (50-100)*2 = -100, (50-200)*2 = -300
        // layout = screen + layout_position = -100+1920 = 1820, -300+0 = -300
        let os = mock_output_state((100.0, 200.0), 2.0, (1920, 0));
        let canvas = Point::from((50.0, 50.0));
        let layout = Srwm::canvas_to_layout_pos(canvas, &os);
        assert!((layout.x - 1820.0).abs() < 0.001);
        assert!((layout.y - (-300.0)).abs() < 0.001);
    }

    #[test]
    fn layout_to_canvas_known_values() {
        // layout=(1920,0), layout_position=(1920,0), zoom=1, camera=(500,300)
        // screen = layout - layout_position = (0, 0)
        // canvas = screen / zoom + camera = 0 + 500 = 500, 0 + 300 = 300
        let os = mock_output_state((500.0, 300.0), 1.0, (1920, 0));
        let layout = Point::from((1920.0, 0.0));
        let canvas = Srwm::layout_to_canvas_pos(layout, &os);
        assert!((canvas.x - 500.0).abs() < 0.001);
        assert!((canvas.y - 300.0).abs() < 0.001);
    }

    #[test]
    fn round_trip_two_outputs_different_cameras() {
        let os_a = mock_output_state((0.0, 0.0), 1.0, (0, 0));
        let os_b = mock_output_state((500.0, 200.0), 0.5, (1920, 0));

        let canvas = Point::from((600.0, 300.0));
        // Through output A
        let layout_a = Srwm::canvas_to_layout_pos(canvas, &os_a);
        let back_a = Srwm::layout_to_canvas_pos(layout_a, &os_a);
        assert!((back_a.x - canvas.x).abs() < 0.001);
        assert!((back_a.y - canvas.y).abs() < 0.001);

        // Through output B
        let layout_b = Srwm::canvas_to_layout_pos(canvas, &os_b);
        let back_b = Srwm::layout_to_canvas_pos(layout_b, &os_b);
        assert!((back_b.x - canvas.x).abs() < 0.001);
        assert!((back_b.y - canvas.y).abs() < 0.001);
    }
}
#[cfg(test)]
mod camera_persistence_tests {
    use super::*;

    #[test]
    fn load_cameras_returns_empty_when_no_file() {
        let tmp = std::env::temp_dir().join(format!("srwm-test-dir-none-{}", std::process::id()));
        let cameras = load_cameras_from(Some(tmp));
        assert!(cameras.is_empty());
    }

    #[test]
    fn load_cameras_round_trip() {
        let tmp = std::env::temp_dir().join(format!("srwm-test-round-trip-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        // Write a cameras.toml manually
        let content = r#"  
["eDP-1"]  
camera_x = -960.0  
camera_y = -540.0  
zoom = 1.0  
  
["HDMI-A-1"]  
camera_x = 200.5  
camera_y = -300.25  
zoom = 0.75  
"#;
        std::fs::write(tmp.join("cameras.toml"), content).unwrap();

        let cameras = load_cameras_from(Some(tmp.clone()));
        assert_eq!(cameras.len(), 2);

        let (cam, zoom) = cameras.get("eDP-1").expect("eDP-1 missing");
        assert!((cam.x - (-960.0)).abs() < 1e-10);
        assert!((cam.y - (-540.0)).abs() < 1e-10);
        assert!((zoom - 1.0).abs() < 1e-10);

        let (cam, zoom) = cameras.get("HDMI-A-1").expect("HDMI-A-1 missing");
        assert!((cam.x - 200.5).abs() < 1e-10);
        assert!((cam.y - (-300.25)).abs() < 1e-10);
        assert!((zoom - 0.75).abs() < 1e-10);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_cameras_handles_corrupt_file() {
        let tmp = std::env::temp_dir().join(format!("srwm-test-corrupt-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("cameras.toml"), "this is not valid toml {{{{").unwrap();
        let cameras = load_cameras_from(Some(tmp.clone()));
        // Should return empty, not panic
        assert!(cameras.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_cameras_handles_empty_file() {
        let tmp = std::env::temp_dir().join(format!("srwm-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("cameras.toml"), "").unwrap();
        let cameras = load_cameras_from(Some(tmp.clone()));
        assert!(cameras.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_cameras_handles_partial_entry() {
        let tmp = std::env::temp_dir().join(format!("srwm-test-partial-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // Missing zoom field
        let content = r#"  
["eDP-1"]  
camera_x = -960.0  
camera_y = -540.0  
"#;
        std::fs::write(tmp.join("cameras.toml"), content).unwrap();
        let cameras = load_cameras_from(Some(tmp.clone()));
        // Should either skip the incomplete entry or use a default zoom
        // Either way, should not panic
        assert!(cameras.is_empty() || cameras.contains_key("eDP-1"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
