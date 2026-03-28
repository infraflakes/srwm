fn main() {
    // Discover include/lib paths via pkg-config
    let x11 = pkg_config::probe_library("x11").unwrap();
    let xft = pkg_config::probe_library("xft").unwrap();
    let xinerama = pkg_config::probe_library("xinerama").unwrap();
    let xrender = pkg_config::probe_library("xrender").unwrap();
    let imlib2 = pkg_config::probe_library("imlib2").unwrap();
    let fontconfig = pkg_config::probe_library("fontconfig").unwrap();
    let freetype = pkg_config::probe_library("freetype2").unwrap();

    // Collect all include paths
    let mut build = cc::Build::new();
    build
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
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-sign-compare");

    // Add all pkg-config include paths to the C compiler
    for lib in [
        &x11,
        &xft,
        &xinerama,
        &xrender,
        &imlib2,
        &fontconfig,
        &freetype,
    ] {
        for path in &lib.include_paths {
            build.include(path);
        }
    }

    build.compile("srwm");

    // Phase 2: Generate Rust FFI bindings from bridge.h
    let mut bindgen_builder = bindgen::Builder::default()
        .header("c-src/bridge.h")
        .allowlist_function("srwm_.*")
        .allowlist_var("running");

    // Add include paths for bindgen/clang too
    for lib in [
        &x11,
        &xft,
        &xinerama,
        &xrender,
        &imlib2,
        &fontconfig,
        &freetype,
    ] {
        for path in &lib.include_paths {
            bindgen_builder = bindgen_builder.clang_arg(format!("-I{}", path.display()));
        }
    }

    let bindings = bindgen::Builder::default()
        .header("c-src/bridge.h")
        .clang_arg("-I/usr/include/freetype2")
        .allowlist_function("srwm_.*")
        .allowlist_var("running")
        .blocklist_function("srwm_handle_key")
        .blocklist_function("srwm_handle_mouse")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // pkg_config::probe_library already emits cargo:rustc-link-lib directives,
    // but we need to ensure they're all linked. Add any that pkg-config might miss:
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
