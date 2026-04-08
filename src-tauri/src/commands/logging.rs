//! Logging management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{LoggingStatus, LogDirection};
use crate::state::AppState;
use ebterm::logger::FileLogger;
use std::path::Path;

#[tauri::command]
pub async fn start_logging(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    file_path: String,
) -> CommandResponse<()> {
    let mut logger = FileLogger::new();

    match logger.start(Path::new(&file_path)) {
        Ok(_) => {
            let mut connections = state.connections.write().await;
            if let Some(ctx) = connections.get_mut(&connection_id) {
                ctx.logger = Some(logger);
                ok(())
            } else {
                err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id))
            }
        }
        Err(e) => {
            err(format!("Failed to start logging: {}", e))
        }
    }
}

#[tauri::command]
pub async fn stop_logging(
    state: tauri::State<'_, AppState>,
    connection_id: String,
) -> CommandResponse<()> {
    let mut connections = state.connections.write().await;

    match connections.get_mut(&connection_id) {
        Some(ctx) => {
            if let Some(mut logger) = ctx.logger.take() {
                match logger.stop() {
                    Ok(_) => ok(()),
                    Err(e) => {
                        ctx.logger = Some(logger);
                        err(format!("Failed to stop logging: {}", e))
                    }
                }
            } else {
                err(format!("LOGGING_NOT_ENABLED: Logging is not enabled for connection '{}'", connection_id))
            }
        }
        None => {
            err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id))
        }
    }
}

#[tauri::command]
pub async fn get_logging_status(
    state: tauri::State<'_, AppState>,
    connection_id: String,
) -> CommandResponse<LoggingStatus> {
    let connections = state.connections.read().await;

    match connections.get(&connection_id) {
        Some(ctx) => {
            if let Some(_logger) = &ctx.logger {
                ok(LoggingStatus {
                    enabled: true,
                    file_path: None,
                    bytes_logged: 0,
                    started_at: None,
                })
            } else {
                ok(LoggingStatus {
                    enabled: false,
                    file_path: None,
                    bytes_logged: 0,
                    started_at: None,
                })
            }
        }
        None => {
            err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id))
        }
    }
}

#[tauri::command]
pub async fn log_data(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    _direction: LogDirection,
    data: Vec<u8>,
) -> CommandResponse<()> {
    let mut connections = state.connections.write().await;

    match connections.get_mut(&connection_id) {
        Some(ctx) => {
            if let Some(logger) = &mut ctx.logger {
                let text = String::from_utf8_lossy(&data);
                let result = logger.write(&text);

                match result {
                    Ok(_) => ok(()),
                    Err(e) => err(format!("Failed to log data: {}", e))
                }
            } else {
                err(format!("LOGGING_NOT_ENABLED: Logging is not enabled for connection '{}'", connection_id))
            }
        }
        None => {
            err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id))
        }
    }
}
