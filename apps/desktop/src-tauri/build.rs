fn main() {
    // The webview assets are embedded from ../dist (`pnpm build`). Make sure
    // the directory exists so `cargo test --workspace` and clippy build
    // without the frontend; the app itself is run through `pnpm tauri`.
    if let Err(e) = std::fs::create_dir_all("../dist") {
        println!("cargo::warning=can't create ../dist: {e}");
    }
    tauri_build::build();
}
