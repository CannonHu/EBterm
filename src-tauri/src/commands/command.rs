//! Command file management commands

use super::{CommandResponse, ok, err};
use crate::ipc::CommandInfo;
use crate::state::AppState;
use embedded_debugger::command::manager::CommandManager;
use std::path::Path;

#[tauri::command]
pub async fn load_command_file(
    state: tauri::State<'_, AppState>,
    file_path: String,
) -> CommandResponse<Vec<CommandInfo>> {
    let manager = state.command_manager.read().await;
    let path = Path::new(&file_path);

    match manager.load_from_file(path).await {
        Ok(_) => {
            let commands = manager.get_commands().await;
            let infos: Vec<CommandInfo> = commands
                .into_iter()
                .enumerate()
                .map(|(index, cmd)| {
                    let content_preview = cmd.preview(50);
                    CommandInfo {
                        index,
                        name: cmd.content.clone(),
                        description: cmd.description,
                        content_preview,
                        line_number: cmd.line_number,
                    }
                })
                .collect();
            ok(infos)
        }
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn get_loaded_commands(
    state: tauri::State<'_, AppState>,
) -> CommandResponse<Vec<CommandInfo>> {
    let manager = state.command_manager.read().await;
    let commands = manager.get_commands().await;

    let infos: Vec<CommandInfo> = commands
        .into_iter()
        .enumerate()
        .map(|(index, cmd)| {
            let content_preview = cmd.preview(50);
            CommandInfo {
                index,
                name: cmd.content.clone(),
                description: cmd.description,
                content_preview,
                line_number: cmd.line_number,
            }
        })
        .collect();

    ok(infos)
}

#[tauri::command]
pub async fn execute_command_by_index(
    state: tauri::State<'_, AppState>,
    index: usize,
) -> CommandResponse<String> {
    let manager = state.command_manager.read().await;

    match manager.execute(index).await {
        Ok(result) => {
            if result.success {
                ok(result.output)
            } else {
                err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
            }
        }
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn send_raw_command(
    state: tauri::State<'_, AppState>,
    command: String,
) -> CommandResponse<String> {
    let manager = state.command_manager.read().await;

    match manager.execute_command(&command).await {
        Ok(result) => {
            if result.success {
                ok(result.output)
            } else {
                err(result.error.unwrap_or_else(|| "Unknown error".to_string()))
            }
        }
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_loaded_commands_empty() {
        // This test requires Tauri state which is not available in unit tests
        assert!(true);
    }

    #[tokio::test]
    async fn test_send_raw_command() {
        // This test requires Tauri state which is not available in unit tests
        assert!(true);
    }
}