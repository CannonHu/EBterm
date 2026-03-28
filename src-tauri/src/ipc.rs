//! Tauri IPC types for frontend-backend communication
//!
//!//! Re-exports lib types with IPC-specific additions.
//! Core types (SerialConfig, DataBits, etc.) are now directly used from lib.

use serde::{Deserialize, Serialize};

// Re-export core types from lib for convenience
pub use embedded_debugger::connection::types::{
    ConnectionStatus, SerialConfig, TelnetConfig, DataBits, Parity, StopBits, FlowControl,
};

/// IPC error type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl IpcError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// IPC connection parameters (matches lib ConnectionConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionParams {
    Serial(SerialConfig),
    Telnet(TelnetConfig),
}

/// Serial port information for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialPortInfo {
    pub port_name: String,
    pub port_type: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

/// Connection statistics for IPC
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
}

/// Log direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogDirection {
    Input,
    Output,
}

/// Logging status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingStatus {
    pub enabled: bool,
    pub file_path: Option<String>,
    pub bytes_logged_input: u64,
    pub bytes_logged_output: u64,
    pub started_at: Option<String>,
}

/// Command information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub index: usize,
    pub name: String,
    pub description: Option<String>,
    pub content_preview: String,
    pub line_number: usize,
}

/// Data received event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReceivedEvent {
    pub connection_id: String,
    pub data: Vec<u8>,
}

/// Status changed event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChangedEvent {
    pub connection_id: String,
    pub status: ConnectionStatus,
}

/// Error occurred event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOccurredEvent {
    pub connection_id: String,
    pub error: IpcError,
}
