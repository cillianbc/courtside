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

/// Where the linker finds libmpv on macOS (video): `MPV_LIB_DIR` if set,
/// else Homebrew's library folder.
fn mpv_search_path() {
    println!("cargo::rerun-if-env-changed=MPV_LIB_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let dirs = match std::env::var("MPV_LIB_DIR") {
        Ok(dir) => vec![dir],
        Err(_) => vec!["/opt/homebrew/lib".into(), "/usr/local/lib".into()],
    };
    for dir in dirs {
        if std::path::Path::new(&dir).join("libmpv.dylib").exists() {
            println!("cargo::rustc-link-search=native={dir}");
            return;
        }
    }
    println!("cargo::warning=libmpv not found: brew install mpv, or set MPV_LIB_DIR");
}
