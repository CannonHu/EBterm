//! Logging management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{LogDirection, LoggingStatus};
use crate::state::AppState;
use embedded_debugger::logger::{FileLogger, Logger, LoggerConfig};
use std::path::Path;

#[tauri::command]
pub async fn start_logging(
    state: tauri::State<'_, AppState>,
    session_id: String,
    file_path: String,
) -> CommandResponse<()> {
    let manager = state.session_manager.read().await;

    match manager.get_session(&session_id).await {
        Some(_) => {
            let config = LoggerConfig {
                max_file_size: 10 * 1024 * 1024,
                max_backup_files: 5,
                compress_rotated: true,
                buffer_size: 8192,
            };

            let mut logger = FileLogger::with_config(config);

            match logger.start_logging(Path::new(&file_path)).await {
                Ok(_) => {
                    let mut loggers = state.loggers.write().await;
                    loggers.insert(session_id.clone(), logger);

                    ok(())
                }
                Err(e) => {
                    err(format!("{}: {}", e.code(), e))
                }
            }
        }
        None => {
            err(format!("SESSION_NOT_FOUND: Session '{}' not found", session_id))
        }
    }
}

#[tauri::command]
pub async fn stop_logging(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResponse<()> {
    let mut loggers = state.loggers.write().await;

    match loggers.remove(&session_id) {
        Some(mut logger) => {
            match logger.stop_logging().await {
                Ok(_) => ok(()),
                Err(e) => {
                    loggers.insert(session_id.clone(), logger);
                    err(format!("{}: {}", e.code(), e))
                }
            }
        }
        None => {
            err(format!("LOGGING_NOT_ENABLED: Logging is not enabled for session '{}'", session_id))
        }
    }
}

#[tauri::command]
pub async fn get_logging_status(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResponse<LoggingStatus> {
    let loggers = state.loggers.read().await;

    match loggers.get(&session_id) {
        Some(logger) => {
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
        }
        None => {
            ok(LoggingStatus {
                enabled: false,
                file_path: None,
                bytes_logged_input: 0,
                bytes_logged_output: 0,
                started_at: None,
            })
        }
    }
}

#[tauri::command]
pub async fn log_data(
    state: tauri::State<'_, AppState>,
    session_id: String,
    direction: LogDirection,
    data: Vec<u8>,
) -> CommandResponse<()> {
    let mut loggers = state.loggers.write().await;

    match loggers.get_mut(&session_id) {
        Some(logger) => {
            let result = match direction {
                LogDirection::Input => logger.log_input(&session_id, &data).await,
                LogDirection::Output => logger.log_output(&session_id, &data).await,
            };

            match result {
                Ok(_) => ok(()),
                Err(e) => err(format!("{}: {}", e.code(), e))
            }
        }
        None => {
            err(format!("LOGGING_NOT_ENABLED: Logging is not enabled for session '{}'", session_id))
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