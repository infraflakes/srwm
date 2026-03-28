#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// C calls these when a dynamic key/mouse binding fires
#[unsafe(no_mangle)]
pub extern "C" fn srwm_handle_key(id: std::ffi::c_int) {
    eprintln!("srwm: key callback {id} (not yet implemented)");
}

#[unsafe(no_mangle)]
pub extern "C" fn srwm_handle_mouse(id: std::ffi::c_int) {
    eprintln!("srwm: mouse callback {id} (not yet implemented)");
}
