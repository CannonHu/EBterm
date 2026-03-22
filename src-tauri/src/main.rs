//! Tauri application entry point for embedded-debugger

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod ipc;
mod state;

use commands::command::*;
use commands::connection::*;
use commands::logging::*;
use commands::session::*;
use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            list_serial_ports,
            connect,
            disconnect,
            get_connection_status,
            write_data,
            write_text,
            list_sessions,
            get_session_info,
            rename_session,
            start_logging,
            stop_logging,
            get_logging_status,
            log_data,
            load_command_file,
            get_loaded_commands,
            execute_command_by_index,
            send_raw_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}