fn main() {
    // The webview assets are embedded from ../dist (`pnpm build`). Make sure
    // the directory exists so `cargo test --workspace` and clippy build
    // without the frontend; the app itself is run through `pnpm tauri`.
    if let Err(e) = std::fs::create_dir_all("../dist") {
        println!("cargo::warning=can't create ../dist: {e}");
    }
    mpv_search_path();
    tauri_build::build();
}

/// Where the linker finds libmpv (video): `MPV_LIB_DIR` if set (any
/// system; on Windows the folder with `mpv.lib` or `libmpv.dll.a`), else
/// Homebrew's library folder on macOS. Linux finds the distribution's
/// libmpv (`libmpv-dev`, `mpv-libs-devel`) on the default path.
fn mpv_search_path() {
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo::rustc-link-search=native={dir}");
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
        if std::path::Path::new(dir).join("libmpv.dylib").exists() {
            println!("cargo::rustc-link-search=native={dir}");
            return;
        }
    }
    println!("cargo::warning=libmpv not found: brew install mpv, or set MPV_LIB_DIR");
}
