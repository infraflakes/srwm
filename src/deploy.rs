const DEFAULT_SRWMRC: &str = include_str!("../config/srwmrc.lua");
const DEFAULT_GENERAL: &str = include_str!("../config/general.lua");
const DEFAULT_CANVAS: &str = include_str!("../config/canvas.lua");
const DEFAULT_KEYBINDINGS: &str = include_str!("../config/keybindings.lua");
const DEFAULT_THEMING: &str = include_str!("../config/theming.lua");
const DEFAULT_BAR: &str = include_str!("../config/bar.lua");
const DEFAULT_STARTUP: &str = include_str!("../config/startup.lua");
const DEFAULT_ENV: &str = include_str!("../config/env.lua");

pub fn deploy_defaults() {
    let dir = crate::config::config_dir();
    if dir.exists() {
        return;
    }
    std::fs::create_dir_all(&dir).ok();
    std::fs::write(dir.join("srwmrc.lua"), DEFAULT_SRWMRC).ok();
    std::fs::write(dir.join("general.lua"), DEFAULT_GENERAL).ok();
    std::fs::write(dir.join("canvas.lua"), DEFAULT_CANVAS).ok();
    std::fs::write(dir.join("keybindings.lua"), DEFAULT_KEYBINDINGS).ok();
    std::fs::write(dir.join("theming.lua"), DEFAULT_THEMING).ok();
    std::fs::write(dir.join("bar.lua"), DEFAULT_BAR).ok();
    std::fs::write(dir.join("startup.lua"), DEFAULT_STARTUP).ok();
    std::fs::write(dir.join("env.lua"), DEFAULT_ENV).ok();
}
