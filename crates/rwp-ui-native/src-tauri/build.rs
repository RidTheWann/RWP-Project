#![allow(missing_docs)]

const APPLICATION_COMMANDS: &[&str] = &[
    "session_start",
    "session_stop",
    "session_status",
    "session_reset",
    "session_history",
    "session_history_load",
    "session_compare",
    "session_processes",
    "session_pick_presentmon_file",
    "session_pick_output_file",
    "session_error_remediation",
];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(APPLICATION_COMMANDS));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("failed to configure Tauri application permissions: {error}");
        std::process::exit(1);
    }
}
