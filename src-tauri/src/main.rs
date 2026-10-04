//! Entry point of the local-sights desktop app. Startup only; the wiring
//! lives in the `local_sights_app` library.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = local_sights_app::run() {
        eprintln!("local-sights failed to start: {error}");
        std::process::exit(1);
    }
}
