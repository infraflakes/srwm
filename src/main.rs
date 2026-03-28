mod ffi;

fn main() {
    // Parse args minimally — just "start" for now
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] != "start" {
        eprintln!("Usage: srwm [start]");
        std::process::exit(1);
    }

    loop {
        unsafe {
            if ffi::srwm_init_display() != 0 {
                eprintln!("srwm: cannot open display");
                std::process::exit(1);
            }

            // TODO: Phase 2B — load Lua config here
            // config::deploy_defaults();
            // config::load();

            ffi::srwm_init_setup();
            ffi::srwm_run(); // blocks until running == 0  

            ffi::srwm_cleanup();

            if ffi::srwm_should_restart() == 0 {
                break;
            }
            // TODO: Phase 2C — clear keybindings here
        }
    }
}
