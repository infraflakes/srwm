use mlua::prelude::*;
use std::collections::VecDeque;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Mutex;

static KEY_CALLBACKS: Mutex<Vec<LuaRegistryKey>> = Mutex::new(Vec::new());
static MOUSE_CALLBACKS: Mutex<Vec<LuaRegistryKey>> = Mutex::new(Vec::new());
static SPAWNED_ONCE: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn load_config(lua: &Lua) -> LuaResult<()> {
    let srwm = lua.create_table()?;

    register_key_api(lua, &srwm)?;
    register_mouse_api(lua, &srwm)?;
    register_window_api(lua, &srwm)?;
    register_workspaces_api(lua, &srwm)?;
    register_workspace_api(lua, &srwm)?;
    register_canvas_api(lua, &srwm)?;
    register_bar_api(lua, &srwm)?;
    register_spawn(lua, &srwm)?;
    register_lifecycle(lua, &srwm)?;

    lua.globals().set("srwm", srwm)?;

    register_include(lua)?;

    let config_dir = config_dir();
    let entry = config_dir.join("srwmrc.lua");
    if entry.exists() {
        let code = std::fs::read_to_string(&entry)
            .map_err(|e| mlua::Error::runtime(format!("read {}: {e}", entry.display())))?;
        lua.load(&code)
            .set_name(entry.to_string_lossy().as_ref())
            .exec()?;
    }
    Ok(())
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("srwm")
}

pub(crate) fn get_key_callbacks() -> std::sync::MutexGuard<'static, Vec<LuaRegistryKey>> {
    KEY_CALLBACKS.lock().unwrap()
}

pub(crate) fn get_mouse_callbacks() -> std::sync::MutexGuard<'static, Vec<LuaRegistryKey>> {
    MOUSE_CALLBACKS.lock().unwrap()
}

fn register_include(lua: &Lua) -> LuaResult<()> {
    let include = lua.create_function(|lua, name: String| {
        let path = config_dir().join(format!("{}.lua", name));
        let code = std::fs::read_to_string(&path)
            .map_err(|e| mlua::Error::runtime(format!("include {}.lua: {}", name, e)))?;
        lua.load(&code)
            .set_name(path.to_string_lossy().as_ref())
            .exec()?;
        Ok(())
    })?;
    lua.globals().set("include", include)?;
    Ok(())
}

fn register_key_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let key_table = lua.create_table()?;

    lua.globals().set("Mod1", 8u32)?;
    lua.globals().set("Mod4", 64u32)?;
    lua.globals().set("Shift", 1u32)?;
    lua.globals().set("Ctrl", 4u32)?;

    let bind_fn = lua.create_function(|lua, (mods, key, cb): (String, String, LuaFunction)| {
        let modmask = parse_modifiers(&mods);
        let keysym = string_to_keysym(&key);
        if keysym == 0 {
            return Err(mlua::Error::runtime(format!("unknown key: {}", key)));
        }

        let reg_key = lua.create_registry_value(cb)?;
        let mut cbs = KEY_CALLBACKS.lock().unwrap();
        let id = cbs.len() as i32;
        cbs.push(reg_key);
        drop(cbs);

        unsafe {
            crate::ffi::srwm_add_keybinding(modmask, keysym.into(), id);
            crate::ffi::srwm_grabkeys();
        }
        Ok(())
    })?;

    key_table.set("bind", bind_fn)?;
    srwm.set("key", key_table)?;
    Ok(())
}

fn parse_modifiers(s: &str) -> u32 {
    let mut mask = 0u32;
    for part in s.split('+') {
        match part.trim() {
            "Shift" => mask |= 1,
            "Ctrl" | "Control" => mask |= 4,
            "Mod1" | "Alt" => mask |= 8,
            "Mod4" | "Super" => mask |= 64,
            "" => {}
            other => eprintln!("srwm: unknown modifier: {}", other),
        }
    }
    mask
}

fn string_to_keysym(name: &str) -> u64 {
    let cname = CString::new(name).unwrap();
    unsafe { crate::ffi::srwm_string_to_keysym(cname.as_ptr()) as u64 }
}

fn register_mouse_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let mouse_table = lua.create_table()?;

    let bind_fn = lua.create_function(
        |lua, (mods, button, target, cb): (String, String, String, LuaFunction)| {
            let modmask = parse_modifiers(&mods);

            let btn = match button.to_lowercase().as_str() {
                "button1" | "left" => 1,
                "button2" | "middle" => 2,
                "button3" | "right" => 3,
                "button4" | "scrollup" => 4,
                "button5" | "scrolldown" => 5,
                _ => {
                    return Err(mlua::Error::runtime(format!("invalid button: {}", button)));
                }
            };

            let click = match target.to_lowercase().as_str() {
                "root" => 1,   // ClkRootWin = 1
                "client" => 0, // ClkClientWin = 0
                _ => {
                    return Err(mlua::Error::runtime(format!(
                        "invalid target: {} (expected root, client)",
                        target
                    )));
                }
            };

            let reg_key = lua.create_registry_value(cb)?;
            let mut cbs = MOUSE_CALLBACKS.lock().unwrap();
            let id = cbs.len() as i32;
            cbs.push(reg_key);
            drop(cbs);

            unsafe {
                crate::ffi::srwm_add_mousebinding(click, modmask, btn, id);
            }
            Ok(())
        },
    )?;

    mouse_table.set("bind", bind_fn)?;
    srwm.set("mouse", mouse_table)?;
    Ok(())
}

fn register_window_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let win = lua.create_table()?;

    win.set(
        "kill",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_killclient();
            }
            Ok(())
        })?,
    )?;

    win.set(
        "toggle_fullscreen",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_togglefullscr();
            }
            Ok(())
        })?,
    )?;

    win.set(
        "focus",
        lua.create_function(|_, dir: i32| {
            unsafe {
                crate::ffi::srwm_action_focusstack(dir);
            }
            Ok(())
        })?,
    )?;

    let border = lua.create_table()?;
    border.set(
        "active",
        lua.create_function(|_, hex: String| {
            let c = CString::new(hex).unwrap();
            unsafe {
                crate::ffi::srwm_set_border_active(c.as_ptr());
            }
            Ok(())
        })?,
    )?;
    border.set(
        "inactive",
        lua.create_function(|_, hex: String| {
            let c = CString::new(hex).unwrap();
            unsafe {
                crate::ffi::srwm_set_border_inactive(c.as_ptr());
            }
            Ok(())
        })?,
    )?;
    win.set("border", border)?;

    srwm.set("window", win)?;
    Ok(())
}

fn register_workspaces_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let ws = lua.create_table()?;

    ws.set(
        "set_label",
        lua.create_function(|_, input: String| {
            let parts: Vec<&str> = input.split(',').collect();
            let count = parts.len().min(9);
            for (i, part) in parts.iter().take(count).enumerate() {
                let c = CString::new(part.trim()).unwrap();
                unsafe {
                    crate::ffi::srwm_set_ws_label(i as i32, c.as_ptr());
                }
            }
            unsafe {
                crate::ffi::srwm_set_ws_count(count as i32);
            }
            Ok(())
        })?,
    )?;

    srwm.set("workspaces", ws)?;
    Ok(())
}

fn register_workspace_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let ws = lua.create_table()?;

    ws.set(
        "view",
        lua.create_function(|_, idx: i32| {
            unsafe {
                crate::ffi::srwm_action_view(idx - 1);
            }
            Ok(())
        })?,
    )?;

    ws.set(
        "move_window_to",
        lua.create_function(|_, idx: i32| {
            unsafe {
                crate::ffi::srwm_action_move_to_ws(idx - 1);
            }
            Ok(())
        })?,
    )?;

    ws.set(
        "shift_view",
        lua.create_function(|_, dir: i32| {
            unsafe {
                crate::ffi::srwm_action_shiftview(dir);
            }
            Ok(())
        })?,
    )?;

    ws.set(
        "view_prev",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_ws_to_prev();
            }
            Ok(())
        })?,
    )?;

    ws.set(
        "view_next",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_ws_to_next();
            }
            Ok(())
        })?,
    )?;

    ws.set(
        "move_to_monitor",
        lua.create_function(|_, dir: i32| {
            unsafe {
                crate::ffi::srwm_action_move_window_to_monitor(dir);
            }
            Ok(())
        })?,
    )?;

    srwm.set("workspace", ws)?;
    Ok(())
}

fn register_canvas_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let canvas = lua.create_table()?;

    canvas.set(
        "move",
        lua.create_function(|_, dir: i32| {
            unsafe {
                crate::ffi::srwm_action_movecanvas(dir);
            }
            Ok(())
        })?,
    )?;

    canvas.set(
        "home",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_homecanvas();
            }
            Ok(())
        })?,
    )?;

    canvas.set(
        "center_window",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_action_centerwindowoncanvas();
            }
            Ok(())
        })?,
    )?;

    canvas.set(
        "zoom",
        lua.create_function(|_, dir: i32| {
            unsafe {
                crate::ffi::srwm_action_zoomcanvas(dir);
            }
            Ok(())
        })?,
    )?;

    srwm.set("canvas", canvas)?;
    Ok(())
}

fn register_bar_api(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    let bar = lua.create_table()?;

    bar.set(
        "fonts",
        lua.create_function(|_, font: String| {
            let c = CString::new(font).unwrap();
            unsafe {
                crate::ffi::srwm_set_font(c.into_raw());
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "tab_height",
        lua.create_function(|_, v: i32| {
            unsafe {
                crate::ffi::srwm_set_tab_height(v);
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "tab_tile_vertical_padding",
        lua.create_function(|_, v: i32| {
            unsafe {
                crate::ffi::srwm_set_tab_tile_vertical_padding(v);
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "tab_tile_inner_padding_horizontal",
        lua.create_function(|_, v: i32| {
            unsafe {
                crate::ffi::srwm_set_tab_tile_inner_padding_horizontal(v);
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "tab_tile_outer_padding_horizontal",
        lua.create_function(|_, v: i32| {
            unsafe {
                crate::ffi::srwm_set_tab_tile_outer_padding_horizontal(v);
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "tab_top",
        lua.create_function(|_, v: bool| {
            unsafe {
                crate::ffi::srwm_set_toptab(if v { 1 } else { 0 });
            }
            Ok(())
        })?,
    )?;

    bar.set(
        "theme",
        lua.create_function(|_, tbl: LuaTable| {
            let two_color_map: &[(&str, i32)] = &[("tab_selected", 0), ("tab_normal", 1)];
            for &(key, scheme) in two_color_map {
                if let Ok(t) = tbl.get::<LuaTable>(key) {
                    if let Ok(fg) = t.get::<String>(1) {
                        let c = CString::new(fg).unwrap();
                        unsafe {
                            crate::ffi::srwm_set_color(scheme, 0, c.as_ptr());
                        }
                    }
                    if let Ok(bg) = t.get::<String>(2) {
                        let c = CString::new(bg).unwrap();
                        unsafe {
                            crate::ffi::srwm_set_color(scheme, 1, c.as_ptr());
                        }
                    }
                }
            }
            let one_color_map: &[(&str, i32)] =
                &[("button_prev", 2), ("button_next", 3), ("button_close", 4)];
            for &(key, scheme) in one_color_map {
                if let Ok(hex) = tbl.get::<String>(key) {
                    let c = CString::new(hex).unwrap();
                    unsafe {
                        crate::ffi::srwm_set_color(scheme, 0, c.as_ptr());
                    }
                }
            }
            Ok(())
        })?,
    )?;

    let ws_sub = lua.create_table()?;
    ws_sub.set(
        "set_label",
        lua.create_function(|_, input: String| {
            let parts: Vec<&str> = input.split(',').collect();
            let count = parts.len().min(9);
            for (i, part) in parts.iter().take(count).enumerate() {
                let c = CString::new(part.trim()).unwrap();
                unsafe {
                    crate::ffi::srwm_set_ws_label(i as i32, c.as_ptr());
                }
            }
            unsafe {
                crate::ffi::srwm_set_ws_count(count as i32);
            }
            Ok(())
        })?,
    )?;
    bar.set("workspaces", ws_sub)?;

    srwm.set("bar", bar)?;
    Ok(())
}

fn register_spawn(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    srwm.set(
        "spawn",
        lua.create_function(|_, cmd: String| {
            std::thread::spawn(move || {
                let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
            });
            Ok(())
        })?,
    )?;

    srwm.set(
        "spawn_once",
        lua.create_function(|_, cmd: String| {
            let mut spawned = SPAWNED_ONCE.lock().unwrap();
            if spawned.contains(&cmd) {
                return Ok(());
            }
            spawned.push_back(cmd.clone());
            drop(spawned);

            std::thread::spawn(move || {
                let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
            });
            Ok(())
        })?,
    )?;
    Ok(())
}

fn register_lifecycle(lua: &Lua, srwm: &LuaTable) -> LuaResult<()> {
    srwm.set(
        "quit",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_quit();
            }
            Ok(())
        })?,
    )?;

    srwm.set(
        "restart",
        lua.create_function(|_, ()| {
            unsafe {
                crate::ffi::srwm_request_restart();
            }
            Ok(())
        })?,
    )?;

    let cfg = lua.create_table()?;
    cfg.set(
        "borderpx",
        lua.create_function(|_, v: u32| {
            unsafe {
                crate::ffi::srwm_set_borderpx(v);
            }
            Ok(())
        })?,
    )?;
    srwm.set("cfg", cfg)?;

    srwm.set(
        "env",
        lua.create_function(|_lua, args: mlua::Variadic<mlua::String>| {
            if args.len() >= 2 {
                let key = args[0].to_str().unwrap().to_string();
                let value = args[1].to_str().unwrap().to_string();
                unsafe {
                    std::env::set_var(&key, &value);
                }
            }
            Ok(mlua::Value::Nil)
        })?,
    )?;

    Ok(())
}
