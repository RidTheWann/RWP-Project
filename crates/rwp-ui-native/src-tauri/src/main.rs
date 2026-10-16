#![allow(missing_docs)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Emitter, Manager};

mod commands;
mod logging;

fn main() -> Result<(), tauri::Error> {
    logging::initialize();
    tracing::info!(target: "rwp", event = "application_started");

    tauri::Builder::default()
        .manage(commands::SessionManager::default())
        .manage(commands::WindowCloseGuard::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::session_start,
            commands::session_stop,
            commands::session_status,
            commands::session_reset,
            commands::session_history,
            commands::session_history_load,
            commands::session_compare,
            commands::session_processes,
            commands::session_pick_presentmon_file,
            commands::session_pick_output_file,
            commands::session_error_remediation,
            commands::session_start_managed,
            commands::session_preflight,
            commands::session_error_kind,
            commands::session_close_confirmed,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let guard = window.state::<commands::WindowCloseGuard>();
                if guard.allow_close.swap(false, std::sync::atomic::Ordering::AcqRel) {
                    return;
                }
                api.prevent_close();
                let _ = window.emit("rwp-window-close-requested", ());
            }
        })
        .run(tauri::generate_context!())
}
