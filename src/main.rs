mod config;
mod deploy;
mod ffi;

fn main() {
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
