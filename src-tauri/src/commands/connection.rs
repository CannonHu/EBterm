//! Connection management commands

use super::{CommandResponse, ok, err};
use crate::ipc::{ConnectionParams, SerialPortInfo};
use crate::state::AppState;

#[tauri::command]
pub async fn list_serial_ports() -> CommandResponse<Vec<SerialPortInfo>> {
    // Use lib layer's discovery function
    match embedded_debugger::connection::discover_serial_ports() {
        Ok(ports) => {
            let port_infos: Vec<SerialPortInfo> = ports
                .into_iter()
                .map(|port| {
                    SerialPortInfo {
                        port_name: port.port_name.clone(),
                        port_type: port.port_type,
                        vendor_id: None,
                        product_id: None,
                        serial_number: None,
                        manufacturer: None,
                        product: Some(port.port_name),
                    }
                })
                .collect();
            ok(port_infos)
        }
        Err(e) => err(format!("Failed to list serial ports: {}", e)),
    }
}

/// Convert IPC params to lib config - NOW DIRECT MAPPING!
/// (Types are identical, just pass through)
fn convert_connection_params(params: ConnectionParams) -> (embedded_debugger::connection::types::ConnectionType, embedded_debugger::connection::types::ConnectionConfig, String) {
    match params {
        ConnectionParams::Serial(serial_params) => {
            let name = serial_params.port.clone();
            (
                embedded_debugger::connection::types::ConnectionType::Serial,
                embedded_debugger::connection::types::ConnectionConfig::Serial(serial_params),
                name,
            )
        }
        ConnectionParams::Telnet(telnet_params) => {
            let name = format!("{}:{}", telnet_params.host, telnet_params.port);
            (
                embedded_debugger::connection::types::ConnectionType::Telnet,
                embedded_debugger::connection::types::ConnectionConfig::Telnet(telnet_params),
                name,
            )
        }
    }
}

#[tauri::command]
pub async fn connect(
    state: tauri::State<'_, AppState>,
    params: ConnectionParams,
) -> CommandResponse<String> {
    let (_, connection_config, connection_name) = convert_connection_params(params);

    let manager = state.connection_manager.read().await;

    match manager.create_connection(connection_name, connection_config).await {
        Ok(connection_id) => ok(connection_id),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn disconnect(
    state: tauri::State<'_, AppState>,
    connection_id: String,
) -> CommandResponse<()> {
    let manager = state.connection_manager.read().await;

    match manager.disconnect(&connection_id).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, AppState>,
    connection_id: String,
) -> CommandResponse<embedded_debugger::connection::types::ConnectionStatus> {
    let manager = state.connection_manager.read().await;

    match manager.get_connection(&connection_id).await {
        Some(connection_info) => {
            let status = match connection_info.status.as_str() {
                "Disconnected" => embedded_debugger::connection::types::ConnectionStatus::Disconnected,
                "Connecting" => embedded_debugger::connection::types::ConnectionStatus::Connecting,
                "Connected" => embedded_debugger::connection::types::ConnectionStatus::Connected,
                "Error" => embedded_debugger::connection::types::ConnectionStatus::Error,
                _ => embedded_debugger::connection::types::ConnectionStatus::Disconnected,
            };
            ok(status)
        }
        None => err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id)),
    }
}

#[tauri::command]
pub async fn write_data(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    data: Vec<u8>,
) -> CommandResponse<()> {
    let manager = state.connection_manager.read().await;

    match manager.write(&connection_id, data).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn write_text(
    state: tauri::State<'_, AppState>,
    connection_id: String,
    text: String,
) -> CommandResponse<()> {
    write_data(state, connection_id, text.into_bytes()).await
}
