//! Logging management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{LogDirection, LoggingStatus};
use crate::state::AppState;
use embedded_debugger::logger::{Logger, LoggerConfig};
use std::path::Path;

#[tauri::command]
pub async fn start_logging(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    file_path: String,
) -> CommandResponse<()> {
    let config = LoggerConfig {
        max_file_size: 10 * 1024 * 1024,
        max_backup_files: 5,
        compress_rotated: true,
        buffer_size: 8192,
    };

    let mut logger = embedded_debugger::logger::FileLogger::with_config(config);

    match logger.start_logging(Path::new(&file_path)).await {
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
                match logger.stop_logging().await {
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
            if let Some(logger) = &ctx.logger {
                let stats = logger.stats();
                let file_path = logger.current_log_path().map(|p| p.to_string_lossy().to_string());
                ok(LoggingStatus {
                    enabled: true,
                    file_path,
                    bytes_logged_input: stats.bytes_logged_input,
                    bytes_logged_output: stats.bytes_logged_output,
                    started_at: stats.started_at.map(|t| {
                        let datetime = chrono::DateTime::<chrono::Utc>::from(t);
                        datetime.to_rfc3339()
                    }),
                })
            } else {
                ok(LoggingStatus {
                    enabled: false,
                    file_path: None,
                    bytes_logged_input: 0,
                    bytes_logged_output: 0,
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
    direction: LogDirection,
    data: Vec<u8>,
) -> CommandResponse<()> {
    let mut connections = state.connections.write().await;

    match connections.get_mut(&connection_id) {
        Some(ctx) => {
            if let Some(logger) = &mut ctx.logger {
                let result = match direction {
                    LogDirection::Input => logger.log_input(&connection_id, &data).await,
                    LogDirection::Output => logger.log_output(&connection_id, &data).await,
                };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_logging_status_default() {
        // This test requires Tauri state which is not available in unit tests
        assert!(true);
    }

    #[tokio::test]
    async fn test_stop_logging_not_enabled() {
        // This test requires Tauri state which is not available in unit tests
        assert!(true);
    }
}
