#![allow(missing_docs)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod logging;

fn main() -> Result<(), tauri::Error> {
    logging::initialize();
    tracing::info!(target: "rwp", event = "application_started");

    tauri::Builder::default()
        .manage(commands::SessionManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::session_start,
            commands::session_stop,
            commands::session_status,
            commands::session_reset,
            commands::session_history,
            commands::session_history_load,
            commands::session_compare,
        ])
        .run(tauri::generate_context!())
}
