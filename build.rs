fn main() {
    // Phase 1: Compile C core into a static library
    cc::Build::new()
        .files(&[
            "c-src/wm.c",
            "c-src/bar.c",
            "c-src/bridge.c",
            "c-src/canvas.c",
            "c-src/drw.c",
            "c-src/events.c",
            "c-src/mouse.c",
            "c-src/setup.c",
            "c-src/util.c",
            "c-src/workspace.c",
        ])
        .include("c-src")
        .define("XINERAMA", None)
        // pkg-config flags for X11 stack
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-sign-compare")
        // Add include paths from pkg-config
        .flag("-I/usr/include/freetype2") // Xft needs this
        .flag("-I/usr/include/Imlib2")
        .compile("srwm");

    // Phase 2: Generate Rust FFI bindings from bridge.h
    let bindings = bindgen::Builder::default()
        .header("c-src/bridge.h")
        .clang_arg("-I/usr/include/freetype2")
        .allowlist_function("srwm_.*")
        .allowlist_var("running")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Link system libraries
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=Xinerama");
    println!("cargo:rustc-link-lib=Xft");
    println!("cargo:rustc-link-lib=Xrender");
    println!("cargo:rustc-link-lib=Imlib2");
    println!("cargo:rustc-link-lib=fontconfig");
    println!("cargo:rustc-link-lib=freetype");

    // Rebuild if C sources change
    println!("cargo:rerun-if-changed=c-src/");
}
