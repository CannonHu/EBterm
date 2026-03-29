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

/// Convert IPC params to lib config
fn convert_connection_params(params: ConnectionParams) -> embedded_debugger::connection::types::ConnectionConfig {
    match params {
        ConnectionParams::Serial(serial_params) => {
            embedded_debugger::connection::types::ConnectionConfig::Serial(serial_params)
        }
        ConnectionParams::Telnet(telnet_params) => {
            embedded_debugger::connection::types::ConnectionConfig::Telnet(telnet_params)
        }
    }
}

#[tauri::command]
pub async fn connect(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    params: ConnectionParams,
) -> CommandResponse<String> {
    let connection_config = convert_connection_params(params);
    let config_clone = connection_config.clone();

    // Create connection handle using factory
    let handle = match connection_config {
        embedded_debugger::connection::types::ConnectionConfig::Serial(serial_config) => {
            embedded_debugger::connection::types::ConnectionFactory::create_serial(serial_config)
        }
        embedded_debugger::connection::types::ConnectionConfig::Telnet(telnet_config) => {
            embedded_debugger::connection::types::ConnectionFactory::create_telnet(telnet_config)
        }
    };

    let handle: embedded_debugger::connection::ConnectionHandle =
        std::sync::Arc::new(tokio::sync::Mutex::new(handle));

    // Connect to the target
    {
        let mut conn = handle.lock().await;
        match conn.connect().await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Connection failed: {} - {}", e.code(), e);
                return err(format!("{}: {}", e.code(), e));
            }
        }
    }

    let connection_id = uuid::Uuid::new_v4().to_string();

    // Create connection context and start streaming
    let mut ctx = crate::connection_context::ConnectionContext::new(handle, config_clone);
    ctx.start_data_streaming(app.clone(), connection_id.clone());

    // Store in connections map
    {
        let mut connections = state.connections.write().await;
        connections.insert(connection_id.clone(), ctx);
    }

    ok(connection_id)
}

#[tauri::command]
pub async fn disconnect(
    state: tauri::State<'_, AppState>,
    params: crate::ipc::DisconnectParams,
) -> CommandResponse<()> {
    let crate::ipc::DisconnectParams { connection_id } = params;

    let mut connections = state.connections.write().await;

    let ctx = match connections.get_mut(&connection_id) {
        Some(ctx) => ctx,
        None => return err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id)),
    };

    // Stop data streaming
    ctx.stop_data_streaming();

    // Disconnect and remove from map
    let mut conn = ctx.handle.lock().await;
    match conn.disconnect().await {
        Ok(_) => {
            drop(conn);
            connections.remove(&connection_id);
            ok(())
        }
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}

#[tauri::command]
pub async fn get_connection_status(
    state: tauri::State<'_, AppState>,
    params: crate::ipc::GetConnectionStatusParams,
) -> CommandResponse<embedded_debugger::connection::types::ConnectionStatus> {
    let crate::ipc::GetConnectionStatusParams { connection_id } = params;

    let connections = state.connections.read().await;

    match connections.get(&connection_id) {
        Some(ctx) => {
            let conn = ctx.handle.lock().await;
            let status = conn.status();
            ok(status)
        }
        None => err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id)),
    }
}

#[tauri::command]
pub async fn write_text(
    state: tauri::State<'_, AppState>,
    params: crate::ipc::WriteTextParams,
) -> CommandResponse<()> {
    let crate::ipc::WriteTextParams { connection_id, text } = params;
    let data = text.into_bytes();

    let connections = state.connections.read().await;

    let ctx = match connections.get(&connection_id) {
        Some(ctx) => ctx,
        None => {
            return err(format!("CONNECTION_NOT_FOUND: Connection '{}' not found", connection_id));
        }
    };

    let mut conn = ctx.handle.lock().await;

    if !conn.is_connected() {
        return err("CONNECTION_NOT_CONNECTED: Connection is not connected".to_string());
    }

    match conn.write(&data).await {
        Ok(_) => ok(()),
        Err(e) => err(format!("{}: {}", e.code(), e)),
    }
}
