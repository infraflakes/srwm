mod backend;
mod dbus;
mod handlers;
mod input;
mod install;
mod render;
mod state;

use crate::dbus::screencasting::Screencasting;
use smithay::reexports::wayland_server::Resource;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;
use state::{ClientState, Srwc};
use std::sync::Arc;

fn has_systemctl() -> bool {
    std::process::Command::new("systemctl")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn update_dbus_activation_env() {
    let session_vars = "WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP XDG_SESSION_CLASS";
    let cmd = format!(
        "hash dbus-update-activation-environment 2>/dev/null && \
         dbus-update-activation-environment {session_vars}"
    );
    match std::process::Command::new("/bin/sh")
        .args(["-c", &cmd])
        .spawn()
    {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => tracing::warn!("Failed to import session environment: {e}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (RUST_LOG=info by default)
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Global flags (work regardless of subcommand)
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("srwc {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Subcommand dispatch
    match std::env::args().nth(1).as_deref() {
        Some("start") => {} // fall through to compositor startup below
        Some("install") => {
            return install::run_install();
        }
        Some("uninstall") => {
            return install::run_uninstall();
        }
        Some("check-config") => {
            let _config = srwc::config::Config::load();
            tracing::info!("Config OK");
            return Ok(());
        }
        _ => {
            println!("srwc {}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Usage: srwc <command> [options]");
            println!();
            println!("Commands:");
            println!("  start          Start the compositor");
            println!("  install        Install session artifacts (desktop file, portals, config)");
            println!("  uninstall      Remove installed session artifacts");
            println!("  check-config   Validate configuration and exit");
            println!();
            println!("Options:");
            println!("  --version, -V  Print version");
            println!();
            println!("Start options:");
            println!("  --backend <winit|udev>  Force backend (default: auto-detect)");
            return Ok(());
        }
    }

    // Parse --backend arg (default: udev on bare metal, winit if nested)
    // This scans all args, so it works after "start" as well.
    let backend_name = std::env::args()
        .skip_while(|a| a != "--backend")
        .nth(1)
        .unwrap_or_else(|| {
            if std::env::var_os("WAYLAND_DISPLAY").is_some()
                || std::env::var_os("DISPLAY").is_some()
            {
                "winit".to_string()
            } else {
                "udev".to_string()
            }
        });

    // Create calloop event loop
    let mut event_loop: smithay::reexports::calloop::EventLoop<Srwc> =
        smithay::reexports::calloop::EventLoop::try_new()?;

    // Create Wayland display
    let display = smithay::reexports::wayland_server::Display::<Srwc>::new()?;

    // Build compositor state
    let mut data = Srwc::new(
        display.handle(),
        event_loop.handle(),
        event_loop.get_signal(),
    );

    // Initialize backend BEFORE setting WAYLAND_DISPLAY.
    let drm_device = match backend_name.as_str() {
        "udev" => Some(backend::udev::init_udev(&mut event_loop, &mut data)?),
        _ => {
            backend::winit::init_winit(&mut event_loop, &mut data)?;
            None
        }
    };

    // Now create listening socket and advertise it to child processes
    // BEFORE D-Bus connections (zbus receiver threads) are initialized.
    let listening_socket = smithay::wayland::socket::ListeningSocketSource::new_auto()?;
    let socket_name = listening_socket
        .socket_name()
        .to_string_lossy()
        .into_owned();
    tracing::info!("Listening on WAYLAND_DISPLAY={socket_name}");
    // Standard Wayland session env vars for child processes.
    // SAFETY: No threads have been spawned yet — Config::load() ran in
    // Srwc::new() and backend init does not start background threads.
    // D-Bus connections (which spawn zbus receiver threads) are created below.
    unsafe {
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
        std::env::set_var("XDG_CURRENT_DESKTOP", "srwc");
        std::env::set_var("XDG_SESSION_CLASS", "user");
        std::env::set_var("XDG_SESSION_DESKTOP", "srwc");
    }

    let is_session = backend_name == "udev";
    let has_systemd = is_session && has_systemctl();
    data.has_systemd = has_systemd;

    // Initialize screencasting + D-Bus ScreenCast service (udev only)
    if backend_name == "udev" {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        // ServiceChannel: gives xdg-desktop-portal-gnome a dedicated Wayland connection
        let (service_tx, service_rx) =
            calloop::channel::channel::<std::os::unix::net::UnixStream>();
        event_loop
            .handle()
            .insert_source(service_rx, |event, _, data: &mut Srwc| {
                if let calloop::channel::Event::Msg(stream) = event {
                    tracing::info!("New service channel client connected");
                    if let Err(e) = data
                        .display_handle
                        .insert_client(stream, std::sync::Arc::new(ClientState::default()))
                    {
                        tracing::warn!("Failed to insert service channel client: {e}");
                    }
                }
            })
            .expect("failed to insert service channel source");

        let service_channel = dbus::mutter_service_channel::ServiceChannel::new(service_tx);
        data.screencast.conn_service_channel = dbus::try_start(service_channel);

        // Initialize screencasting subsystem (creates the PipeWire calloop channel)
        data.screencast.screencasting = Some(Screencasting::new(&event_loop.handle()));

        // Create D-Bus calloop channel for ScreenCast messages
        let (to_srwc, from_screen_cast) = smithay::reexports::calloop::channel::channel();
        event_loop
            .handle()
            .insert_source(from_screen_cast, move |event, _, state| match event {
                smithay::reexports::calloop::channel::Event::Msg(msg) => {
                    state.on_screen_cast_msg(msg)
                }
                smithay::reexports::calloop::channel::Event::Closed => (),
            })
            .unwrap();

        // Build the output map for the D-Bus interface
        let ipc_outputs = Arc::new(Mutex::new(HashMap::new()));
        data.screencast.ipc_outputs = Some(ipc_outputs.clone());

        // Backfill outputs created during init_udev() before ipc_outputs was set.
        for output in data.space.outputs() {
            let mode = output.current_mode().unwrap();
            let transform = output.current_transform();
            let size = transform.transform_size(mode.size);
            let pos = output.current_location();
            ipc_outputs.lock().unwrap().insert(
                output.name(),
                dbus::mutter_screen_cast::OutputInfo {
                    name: output.name(),
                    x: pos.x,
                    y: pos.y,
                    width: size.w as u32,
                    height: size.h as u32,
                },
            );
        }

        // Create and start the ScreenCast D-Bus service
        let screen_cast = dbus::ScreenCast::new(ipc_outputs.clone(), to_srwc);
        data.screencast.conn_screen_cast = dbus::start_screen_cast(screen_cast);

        // DisplayConfig — shares the same ipc_outputs Arc as ScreenCast
        let display_config = dbus::mutter_display_config::DisplayConfig::new(ipc_outputs.clone());
        data.screencast.conn_display_config = dbus::try_start(display_config);

        // Introspect — window list for the portal picker
        let (introspect_tx, introspect_rx) = calloop::channel::channel();
        let (to_introspect, from_srwc) = async_channel::unbounded();
        event_loop
            .handle()
            .insert_source(introspect_rx, {
                let to_introspect = to_introspect.clone();
                move |event, _, data: &mut Srwc| {
                    if let calloop::channel::Event::Msg(
                        dbus::gnome_shell_introspect::IntrospectToSrwc::GetWindows,
                    ) = event
                    {
                        let mut windows = HashMap::new();
                        // Iterate over all windows in the space
                        use smithay::wayland::seat::WaylandFocus;
                        for window in data.space.elements() {
                            if let Some(surface) = window.wl_surface() {
                                let id = u64::from(surface.id().protocol_id());
                                smithay::wayland::compositor::with_states(&surface, |states| {
                                    if let Some(data) =
                                        states.data_map.get::<XdgToplevelSurfaceData>()
                                    {
                                        let attrs = data.lock().unwrap();
                                        windows.insert(
                                            id,
                                            dbus::gnome_shell_introspect::WindowProperties {
                                                title: attrs.title.clone().unwrap_or_default(),
                                                app_id: attrs
                                                    .app_id
                                                    .clone()
                                                    .map(|id| format!("{id}.desktop"))
                                                    .unwrap_or_default(),
                                            },
                                        );
                                    }
                                });
                            }
                        }
                        let _ = to_introspect.send_blocking(
                            dbus::gnome_shell_introspect::SrwcToIntrospect::Windows(windows),
                        );
                    }
                }
            })
            .expect("failed to insert introspect source");

        let introspect = dbus::gnome_shell_introspect::Introspect::new(introspect_tx, from_srwc);
        data.screencast.conn_introspect = dbus::try_start(introspect);
    }

    // Register the Wayland Display as a calloop source so client messages
    // are dispatched automatically. This replaces the old poll_fd approach.
    let display_source = smithay::reexports::calloop::generic::Generic::new(
        display,
        smithay::reexports::calloop::Interest::READ,
        smithay::reexports::calloop::Mode::Level,
    );
    event_loop
        .handle()
        .insert_source(display_source, |_, display, data: &mut Srwc| {
            // SAFETY: we never drop the Display while the Generic source is alive
            unsafe { display.get_mut() }.dispatch_clients(data).ok();
            Ok(smithay::reexports::calloop::PostAction::Continue)
        })?;

    // Initialize screencasting + D-Bus ScreenCast service (udev only)
    if has_systemd {
        // systemd-specific: start graphical-session targets via transient anchor
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "start", "graphical-session-pre.target"])
            .status();

        let _ = std::process::Command::new("systemd-run")
            .args([
                "--user",
                "--unit=srwc-session.service",
                "--property=BindsTo=graphical-session.target",
                "--property=After=graphical-session-pre.target",
                "--remain-after-exit",
                "true",
            ])
            .status();

        // Import specific session vars to systemd
        let session_vars = "WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP XDG_SESSION_CLASS";
        let cmd = format!("systemctl --user import-environment {session_vars}");
        let _ = std::process::Command::new("/bin/sh")
            .args(["-c", &cmd])
            .spawn()
            .inspect_err(|e| tracing::warn!("Failed to import session environment to systemd: {e}"))
            .and_then(|mut c| c.wait());
        // Also update D-Bus activation environment
        update_dbus_activation_env();
    } else if is_session {
        // Non-systemd: only update D-Bus activation environment
        update_dbus_activation_env();
        tracing::info!("systemd not found, skipping graphical-session target setup");
    }

    event_loop
        .handle()
        .insert_source(listening_socket, |stream, _, data: &mut Srwc| {
            tracing::info!("New client connected");
            if let Err(e) = data
                .display_handle
                .insert_client(stream, Arc::new(ClientState::default()))
            {
                tracing::warn!("Failed to insert client: {e}");
            }
        })?;

    // Config file watcher: poll mtime every 500ms
    {
        let config_path = srwc::config::config_path();
        data.config_file_mtime = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();

        let timer = smithay::reexports::calloop::timer::Timer::from_duration(
            std::time::Duration::from_millis(500),
        );
        event_loop
            .handle()
            .insert_source(timer, move |_, _, data: &mut Srwc| {
                let current_mtime = std::fs::metadata(&config_path)
                    .and_then(|m| m.modified())
                    .ok();
                if current_mtime != data.config_file_mtime && current_mtime.is_some() {
                    // Debounce: skip if mtime is <100ms old (editor may still be writing)
                    let dominated_by_recent_write = current_mtime
                        .is_some_and(|mt| mt.elapsed().is_ok_and(|age| age.as_millis() < 100));
                    if !dominated_by_recent_write {
                        data.config_file_mtime = current_mtime;
                        data.reload_config();
                    }
                }
                smithay::reexports::calloop::timer::TimeoutAction::ToDuration(
                    std::time::Duration::from_millis(500),
                )
            })?;
    }

    // Spawn XWayland (after WAYLAND_DISPLAY is set so it can connect as a client)
    if data.config.xwayland_enabled {
        backend::spawn_xwayland(&data.display_handle, &event_loop.handle());
    }

    // Auto-reap child processes — prevents zombies from exec/autostart commands.
    // Must be after backend init: libseat uses waitpid() during session setup.
    unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN) };

    // Defer autostart until the event loop is running — GTK apps (swaync) need
    // the compositor processing Wayland events before they connect.
    let autostart = data.autostart.clone();
    if !autostart.is_empty() {
        event_loop.handle().insert_source(
            smithay::reexports::calloop::timer::Timer::from_duration(
                std::time::Duration::from_millis(100),
            ),
            move |_, _, _data| {
                for cmd in &autostart {
                    tracing::info!("Autostart: {cmd}");
                    state::spawn_command(cmd);
                }
                smithay::reexports::calloop::timer::TimeoutAction::Drop
            },
        )?;
    }

    // Run the event loop
    tracing::info!("Starting event loop — launch apps with: WAYLAND_DISPLAY={socket_name} <app>");
    event_loop.run(None, &mut data, |data| {
        if let Some(ref device) = drm_device {
            backend::udev::render_if_needed(device, data);
        }
        data.space.refresh();
        data.popups.cleanup();
        data.display_handle.flush_clients().ok();
    })?;

    // Save camera state on exit (fallback for non-Quit exits)
    data.save_cameras();

    if has_systemd {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "srwc-session.service"])
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "unset-environment", "WAYLAND_DISPLAY", "DISPLAY"])
            .status();
    } else if is_session {
        // Non-systemd teardown: just clean up D-Bus env if possible
        let _ = std::process::Command::new("/bin/sh")
            .args([
                "-c",
                "hash dbus-update-activation-environment 2>/dev/null && \
                 dbus-update-activation-environment WAYLAND_DISPLAY= DISPLAY=",
            ])
            .status();
    }

    Ok(())
}
