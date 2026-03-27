//! Tauri application entry point for embedded-debugger

mod commands;
mod data_streamer;
mod ipc;
mod state;

use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::Manager;

use commands::command::*;
use commands::connection::*;
use commands::logging::*;
use state::AppState;

use embedded_debugger::command::manager::DefaultCommandManager;
use embedded_debugger::command::parser::DefaultCommandParser;
use embedded_debugger::connection::ConnectionManager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_handle = app.handle();

            let data_streamer_manager = Arc::new(data_streamer::DataStreamerManager::new(app_handle.clone()));
            let parser = Box::new(DefaultCommandParser::default());
            let command_manager = Arc::new(RwLock::new(DefaultCommandManager::new(parser)));
            let connection_manager = Arc::new(RwLock::new(ConnectionManager::new()));
            let loggers = Arc::new(RwLock::new(std::collections::HashMap::new()));

            app.manage(AppState {
                connection_manager,
                command_manager,
                loggers,
                data_streamer_manager,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_serial_ports,
            connect,
            disconnect,
            get_connection_status,
            write_data,
            write_text,
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
