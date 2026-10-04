//! Build script: generates the Tauri context and an app manifest that turns
//! each custom command into an explicit permission, so the capability file
//! decides which commands the window may invoke.

fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_session",
            "update_input",
            "start_fetch",
            "select_profile",
            "select_region",
            "confirm_connection_change",
            "cancel_connection_change",
            "reload_log_groups",
            "update_log_group_filter",
            "select_log_group",
        ]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("tauri-build failed: {error}");
        std::process::exit(1);
    }
}
