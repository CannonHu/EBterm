//! Tauri application entry point for embedded-debugger

mod connection_context;
mod commands;
mod data_streamer;
mod ipc;
mod state;

use tauri::Manager;

use commands::connection::*;
use commands::logging::*;
use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle();
            app.manage(AppState::new(app_handle.clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_serial_ports,
            connect,
            disconnect,
            get_connection_status,
            write_text,
            start_logging,
            stop_logging,
            get_logging_status,
            log_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
