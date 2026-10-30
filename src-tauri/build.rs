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
            "get_rows",
            "set_failure_list_open",
            "select_time_zone",
            "set_log_filter",
            "open_settings",
            "cancel_settings",
            "save_settings",
            "clear_cache",
            "row_positions",
            "cancel_close",
            "confirm_close",
            "start_sso_login",
            "cancel_sso_login",
            "dismiss_sso_login",
            "open_sso_url",
            "cancel_fetch",
            // 261017-insights-query U1: the Log Analytics screen.
            "update_analytics_input",
            "run_query",
            "cancel_query",
            "get_query_rows",
            // 261017-insights-query U2: an opened row and completion.
            "get_query_row_detail",
            "get_completion_candidates",
            // 261025-insights-stats U2: the words of a query for the screen.
            "get_query_catalog",
            // 261022-small-features U2: the export of either screen.
            "start_export",
            "dismiss_export_notice",
        ]));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("tauri-build failed: {error}");
        std::process::exit(1);
    }
}
