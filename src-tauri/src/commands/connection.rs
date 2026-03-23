//! Connection management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{ConnectionParams, ConnectionStatus as IpcConnectionStatus, SerialPortInfo};
use crate::state::AppState;
use embedded_debugger::connection::{ConnectionConfig, ConnectionType, SerialConfig, TelnetConfig};
use embedded_debugger::connection::types::{DataBits, FlowControl, Parity, StopBits};
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
pub async fn list_serial_ports() -> CommandResponse<Vec<SerialPortInfo>> {
    match serial2::SerialPort::available_ports() {
        Ok(ports) => {
            let port_infos: Vec<SerialPortInfo> = ports
                .into_iter()
                .map(|port| {
                    let port_name = port.to_string_lossy().to_string();
                    SerialPortInfo {
                        port_name: port_name.clone(),
                        port_type: "serial".to_string(),
                        vendor_id: None,
                        product_id: None,
                        serial_number: None,
                        manufacturer: None,
                        product: Some(port_name),
                    }
                })
                .collect();
            ok(port_infos)
        }
        Err(e) => err(format!("Failed to list serial ports: {}", e)),
    }
}

fn convert_connection_params(params: ConnectionParams) -> (ConnectionType, ConnectionConfig, String) {
    match params {
        ConnectionParams::Serial(serial_params) => {
            let config = SerialConfig {
                port: serial_params.port.clone(),
                baud_rate: serial_params.baud_rate,
                data_bits: match serial_params.data_bits {
                    crate::ipc::DataBits::Seven => DataBits::Seven,
                    crate::ipc::DataBits::Eight => DataBits::Eight,
                },
                parity: match serial_params.parity {
                    crate::ipc::Parity::None => Parity::None,
                    crate::ipc::Parity::Odd => Parity::Odd,
                    crate::ipc::Parity::Even => Parity::Even,
                },
                stop_bits: match serial_params.stop_bits {
                    crate::ipc::StopBits::One => StopBits::One,
                    crate::ipc::StopBits::Two => StopBits::Two,
                },
                flow_control: match serial_params.flow_control {
                    crate::ipc::FlowControl::None => FlowControl::None,
                    crate::ipc::FlowControl::Software => FlowControl::Software,
                    crate::ipc::FlowControl::Hardware => FlowControl::Hardware,
                },
            };
            (ConnectionType::Serial, ConnectionConfig::Serial(config), serial_params.port.clone())
        }
        ConnectionParams::Telnet(telnet_params) => {
            let config = TelnetConfig {
                host: telnet_params.host.clone(),
                port: telnet_params.port,
                connect_timeout_secs: telnet_params.connect_timeout_secs,
            };
            let name = format!("{}:{}", telnet_params.host, telnet_params.port);
            (ConnectionType::Telnet, ConnectionConfig::Telnet(config), name)
        }
    }
}

#[tauri::command]
pub async fn connect(
    state: tauri::State<'_, AppState>,
    params: ConnectionParams,
) -> CommandResponse<String> {
    let (connection_type, connection_config, connection_name) = convert_connection_params(params);
    let session_name = connection_name.clone();

    let manager = state.session_manager.read().await;

    match manager.create_session(session_name, connection_type, connection_config).await {
        Ok(session_id) => ok(session_id),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn disconnect(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResponse<()> {
    let manager = state.session_manager.read().await;

    match manager.close_session(&session_id).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CommandResponse<IpcConnectionStatus> {
    let manager = state.session_manager.read().await;

    match manager.get_session(&session_id).await {
        Some(session) => {
            let status = session_state_to_ipc(session.state());
            ok(status)
        }
        None => err(format!("SESSION_NOT_FOUND: Session '{}' not found", session_id)),
    }
}

#[tauri::command]
pub async fn write_data(
    state: tauri::State<'_, AppState>,
    session_id: String,
    data: Vec<u8>,
) -> CommandResponse<()> {
    let manager = state.session_manager.read().await;

    match manager.write_session_data(&session_id, data).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn write_text(
    state: tauri::State<'_, AppState>,
    session_id: String,
    text: String,
) -> CommandResponse<()> {
    write_data(state, session_id, text.into_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandResult;

    #[tokio::test]
    async fn test_list_serial_ports() {
        let result = list_serial_ports().await;
        assert!(result.is_ok());
        let command_result = result.unwrap();
        assert!(command_result.success);
    }

    #[tokio::test]
    async fn test_command_result_helpers() {
        let success_result: CommandResult<i32> = CommandResult::success(42);
        assert!(success_result.success);
        assert_eq!(success_result.data, Some(42));
        assert!(success_result.error.is_none());

        let error_result: CommandResult<i32> = CommandResult::error("Something went wrong");
        assert!(!error_result.success);
        assert!(error_result.data.is_none());
        assert_eq!(error_result.error, Some("Something went wrong".to_string()));
    }
}
