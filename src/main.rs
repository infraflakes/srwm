mod backend;
mod dbus;
mod decorations;
mod focus;
mod grabs;
mod handlers;
mod input;
mod install;
mod render;
mod screencasting;
mod screenshot_ui;
mod state;

use state::{ClientState, Srwm};
use std::sync::Arc;

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
        println!("srwm {}", env!("CARGO_PKG_VERSION"));
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
            let _config = srwm::config::Config::load();
            tracing::info!("Config OK");
            return Ok(());
        }
        _ => {
            println!("srwm {}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Usage: srwm <command> [options]");
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
    let mut event_loop: smithay::reexports::calloop::EventLoop<Srwm> =
        smithay::reexports::calloop::EventLoop::try_new()?;

    // Create Wayland display
    let display = smithay::reexports::wayland_server::Display::<Srwm>::new()?;

    // Build compositor state
    let mut data = Srwm::new(
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

    // Initialize screencasting + D-Bus ScreenCast service (udev only)
    if backend_name == "udev" {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};

        // Initialize screencasting subsystem (creates the PipeWire calloop channel)
        data.screencasting = Some(screencasting::Screencasting::new(&event_loop.handle()));

        // Create D-Bus calloop channel for ScreenCast messages
        let (to_srwm, from_screen_cast) = smithay::reexports::calloop::channel::channel();
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

        // Create and start the ScreenCast D-Bus service
        let screen_cast = dbus::ScreenCast::new(ipc_outputs, to_srwm);
        data.conn_screen_cast = dbus::start_screen_cast(screen_cast);
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
        .insert_source(display_source, |_, display, data: &mut Srwm| {
            // SAFETY: we never drop the Display while the Generic source is alive
            unsafe { display.get_mut() }.dispatch_clients(data).ok();
            Ok(smithay::reexports::calloop::PostAction::Continue)
        })?;

    // Now create listening socket and advertise it to child processes
    let listening_socket = smithay::wayland::socket::ListeningSocketSource::new_auto()?;
    let socket_name = listening_socket
        .socket_name()
        .to_string_lossy()
        .into_owned();
    tracing::info!("Listening on WAYLAND_DISPLAY={socket_name}");
    // Standard Wayland session env vars for child processes
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &socket_name) };
    unsafe { std::env::set_var("XDG_SESSION_TYPE", "wayland") };
    unsafe { std::env::set_var("XDG_CURRENT_DESKTOP", "srwm") };
    // Toolkit env vars (MOZ_ENABLE_WAYLAND, QT_QPA_PLATFORM, etc.) are now
    // set in Config::load() with user [env] overrides taking precedence.
    unsafe { std::env::set_var("XDG_SESSION_CLASS", "user") };
    unsafe { std::env::set_var("XDG_SESSION_DESKTOP", "srwm") };

    let is_session = backend_name == "udev";
    if is_session {
        // Start graphical-session-pre.target
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "start", "graphical-session-pre.target"])
            .status();

        // Create a transient anchor service that keeps graphical-session.target alive.
        // BindsTo= means this service requires the target, so:
        //   1. Starting this service also starts graphical-session.target
        //   2. The target stays alive because this service "needs" it (StopWhenUnneeded won't trigger)
        // --remain-after-exit keeps the service in "active" state after /bin/true exits.
        let _ = std::process::Command::new("systemd-run")
            .args([
                "--user",
                "--unit=srwm-session.service",
                "--property=BindsTo=graphical-session.target",
                "--property=After=graphical-session-pre.target",
                "--remain-after-exit",
                "/bin/true",
            ])
            .status();

        // Import full environment AFTER targets are alive, so D-Bus-activated
        // services find graphical-session.target active.
        let cmd = "systemctl --user import-environment; \
           hash dbus-update-activation-environment 2>/dev/null && \
           dbus-update-activation-environment WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_SESSION_DESKTOP";
        match std::process::Command::new("/bin/sh")
            .args(["-c", cmd])
            .spawn()
        {
            Ok(mut child) => {
                let _ = child.wait();
            }
            Err(e) => tracing::warn!("Failed to import session environment: {e}"),
        }
    }

    event_loop
        .handle()
        .insert_source(listening_socket, |stream, _, data: &mut Srwm| {
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
        let config_path = srwm::config::config_path();
        data.config_file_mtime = std::fs::metadata(&config_path)
            .and_then(|m| m.modified())
            .ok();

        let timer = smithay::reexports::calloop::timer::Timer::from_duration(
            std::time::Duration::from_millis(500),
        );
        event_loop
            .handle()
            .insert_source(timer, move |_, _, data: &mut Srwm| {
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

    if is_session {
        // Stop the transient anchor service. This releases graphical-session.target,
        // which then deactivates (StopWhenUnneeded=yes), cascading to stop portal
        // services and other dependents.
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "srwm-session.service"])
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "unset-environment", "WAYLAND_DISPLAY", "DISPLAY"])
            .status();
    }

    Ok(())
}
