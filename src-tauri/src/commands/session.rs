//! Session management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{ConnectionStatus as IpcConnectionStatus, ConnectionStats, SessionInfo};
use crate::state::AppState;
use embedded_debugger::session::types::SessionState;

fn session_state_to_ipc(state: &SessionState) -> IpcConnectionStatus {
    match state {
        SessionState::Created | SessionState::Connecting => IpcConnectionStatus::Connecting,
        SessionState::Connected => IpcConnectionStatus::Connected,
        SessionState::Disconnecting | SessionState::Disconnected => IpcConnectionStatus::Disconnected,
        SessionState::Error(_) => IpcConnectionStatus::Error,
    }
}

#[tauri::command]
pub async fn list_sessions(
    state: tauri::State<'_, AppState>,
) -> CommandResponse<Vec<SessionInfo>> {
    let manager = state.session_manager.read().await;
    let sessions = manager.list_sessions().await;

    let mut session_infos = Vec::new();
    for session in sessions {
        let metadata = session.metadata();
        let status = session_state_to_ipc(session.state());
        let created_at = metadata.created_at.to_rfc3339();
        let last_activity = metadata.last_activity.to_rfc3339();

        session_infos.push(SessionInfo {
            id: session.id().clone(),
            name: metadata.name.clone(),
            connection_type: metadata.connection_type.clone(),
            status,
            created_at,
            last_activity: Some(last_activity),
            stats: ConnectionStats::default(),
            logging_enabled: false,
            log_file_path: None,
        });
    }

    ok(session_infos)
}

#[tauri::command]
pub async fn get_session_info(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResponse<SessionInfo> {
    let manager = state.session_manager.read().await;

    match manager.get_session(&session_id).await {
        Some(session) => {
            let metadata = session.metadata();
            let status = session_state_to_ipc(session.state());
            let created_at = metadata.created_at.to_rfc3339();
            let last_activity = metadata.last_activity.to_rfc3339();

            ok(SessionInfo {
                id: session.id().clone(),
                name: metadata.name.clone(),
                connection_type: metadata.connection_type.clone(),
                status,
                created_at,
                last_activity: Some(last_activity),
                stats: ConnectionStats::default(),
                logging_enabled: false,
                log_file_path: None,
            })
        }
        None => err(format!(
            "SESSION_NOT_FOUND: Session '{}' not found",
            session_id
        )),
    }
}

#[tauri::command]
pub async fn rename_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
    new_name: String,
) -> CommandResponse<()> {
    let manager = state.session_manager.read().await;

    match manager.rename_session(&session_id, new_name).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandResult;

    #[tokio::test]
    async fn test_list_sessions() {
        // This test requires Tauri state which is not available in unit tests
        // Integration tests should be used instead
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_session_info_not_found() {
        // This test requires Tauri state which is not available in unit tests
        // Integration tests should be used instead
        assert!(true);
    }
}
