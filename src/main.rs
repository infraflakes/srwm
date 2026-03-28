pub mod cli;
pub mod config;
pub mod dbus;
pub mod deploy;
pub mod ffi;
pub mod ipc;
pub mod xserver;

use crate::cli::Cli;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};

static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);

fn main() {
    setup_signal_handlers();

    let cli = Cli::parse();
    match cli.command {
        Some(crate::cli::Command::Start) | None => {
            main_run();
        }
        Some(crate::cli::Command::Version) => {
            println!("srwm {}", env!("CARGO_PKG_VERSION"));
        }
        Some(crate::cli::Command::Ipc { command }) => {
            let full_cmd = format!("{}\n", command);
            if let Err(e) = ipc::send_command(&full_cmd) {
                eprintln!("srwm: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn setup_signal_handlers() {
    use signal_hook::consts::signal::*;
    use signal_hook::low_level::register;

    unsafe {
        let _ = register(SIGTERM, || {
            SHOULD_QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
            crate::ffi::srwm_quit();
        });

        let _ = register(SIGINT, || {
            SHOULD_QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
            crate::ffi::srwm_quit();
        });

        let _ = register(SIGHUP, || {
            crate::ffi::srwm_request_restart();
        });
    }
}

pub fn main_run() {
    let _dbus = dbus::Session::start();

    let xserver: Option<_> = if std::env::var("DISPLAY").is_err() {
        match xserver::start() {
            Ok(srv) => Some(srv),
            Err(e) => {
                eprintln!("srwm: failed to start X server: {}", e);
                None
            }
        }
    } else {
        None
    };

    ipc::start_ipc_server();

    loop {
        deploy::deploy_defaults();

        unsafe {
            if ffi::srwm_init_display() != 0 {
                eprintln!("srwm: cannot open display");
                std::process::exit(1);
            }

            let lua = mlua::Lua::new();
            ffi::set_lua_vm(&lua);

            if let Err(e) = config::load_config(&lua) {
                eprintln!("srwm: lua config error: {}", e);
            }

            ffi::srwm_init_setup();
            ffi::srwm_run();

            ffi::clear_lua_vm();
            ffi::srwm_cleanup();

            config::get_key_callbacks().clear();
            config::get_mouse_callbacks().clear();

            if ffi::srwm_should_restart() == 0 {
                break;
            }

            ffi::srwm_clear_keybindings();
            ffi::srwm_clear_mousebindings();
        }
    }
}
