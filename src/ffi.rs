#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// C calls these when a dynamic key/mouse binding fires
#[no_mangle]
pub extern "C" fn srwm_handle_key(id: std::ffi::c_int) {
    // TODO: dispatch to Lua callback registry
    eprintln!("srwm: key callback {id} (not yet implemented)");
}

#[no_mangle]
pub extern "C" fn srwm_handle_mouse(id: std::ffi::c_int) {
    // TODO: dispatch to Lua callback registry
    eprintln!("srwm: mouse callback {id} (not yet implemented)");
}
