//! Build script: generates the Tauri context and an app manifest that turns
//! each custom command into an explicit permission, so the capability file
//! decides which commands the window may invoke.

fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_session",
            "update_input",
            "start_fetch",
        ]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("tauri-build failed: {error}");
        std::process::exit(1);
    }
}
