//! Tauri application entry point for embedded-debugger

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

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
use commands::session::*;
use state::AppState;

use embedded_debugger::command::manager::DefaultCommandManager;
use embedded_debugger::command::parser::DefaultCommandParser;
use embedded_debugger::logger::FileLogger;
use embedded_debugger::session::manager::{SessionCallbacks, SessionManager};
use embedded_debugger::session::types::SessionManagerConfig;

use crate::data_streamer::{DataStreamerConfig, DataStreamerManager};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            
            // Create DataStreamerManager first (needed for callbacks)
            let data_streamer_manager =
                Arc::new(DataStreamerManager::new(app_handle.clone(), DataStreamerConfig::default()));
            
            // Create SessionCallbacks with DataStreamerManager
            let session_callbacks = SessionCallbacks::new()
                .with_on_connected({
                    let dsm = data_streamer_manager.clone();
                    move |session_id, registry| {
                        let dsm = dsm.clone();
                        tokio::spawn(async move {
                            dsm.start_streaming(session_id, registry).await;
                        });
                    }
                })
                .with_on_disconnected({
                    let dsm = data_streamer_manager.clone();
                    move |session_id| {
                        let dsm = dsm.clone();
                        tokio::spawn(async move {
                            dsm.stop_streaming(&session_id).await;
                        });
                    }
                })
                .with_on_error(|session_id, error| {
                    tracing::error!("Session {} error: {}", session_id, error);
                });
            
            // Create SessionManager with callbacks (single initialization)
            let session_manager = Arc::new(RwLock::new(
                SessionManager::with_config_and_callbacks(
                    SessionManagerConfig::default(),
                    session_callbacks,
                )
            ));
            
            // Create other managers
            let parser = Box::new(DefaultCommandParser::default());
            let command_manager = Arc::new(RwLock::new(DefaultCommandManager::new(parser)));
            let loggers: Arc<RwLock<std::collections::HashMap<String, FileLogger>>> = 
                Arc::new(RwLock::new(std::collections::HashMap::new()));
            
            // Manage the complete AppState
            app.manage(AppState {
                session_manager,
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