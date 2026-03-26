//! Tauri IPC types for frontend-backend communication
//!
//! Defines all data structures for commands and events

use serde::{Deserialize, Serialize};

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

/// Write operation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteError {
    Disconnected,
    Timeout,
    IoError(String),
}

/// Connection parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionParams {
    Serial(SerialParams),
    Telnet(TelnetParams),
}

/// Serial connection parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialParams {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
}

impl Default for SerialParams {
    fn default() -> Self {
        Self {
            port: String::new(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        }
    }
}

/// Telnet connection parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelnetParams {
    pub host: String,
    pub port: u16,
    pub connect_timeout_secs: u64,
}

impl Default for TelnetParams {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 23,
            connect_timeout_secs: 10,
        }
    }
}

/// Data bits
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataBits {
    Seven,
    Eight,
}

/// Parity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    None,
    Odd,
    Even,
}

/// Stop bits
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopBits {
    One,
    Two,
}

/// Flow control
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

/// Connection status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub connection_type: String,
    pub status: ConnectionStatus,
    pub created_at: String,
    pub last_activity: Option<String>,
    pub stats: ConnectionStats,
    pub logging_enabled: bool,
    pub log_file_path: Option<String>,
}

/// Connection statistics
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

impl std::fmt::Display for LogDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogDirection::Input => write!(f, "Input"),
            LogDirection::Output => write!(f, "Output"),
        }
    }
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

/// Serial port information
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

/// Log entry for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub session_id: String,
    pub direction: LogDirection,
    pub data: Vec<u8>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(session_id: impl Into<String>, direction: LogDirection, data: Vec<u8>) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            session_id: session_id.into(),
            direction,
            data,
        }
    }

    /// Get the data as a hex string
    pub fn data_as_hex(&self) -> String {
        self.data.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Get the data as a string (lossy conversion)
    pub fn data_as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_error_creation() {
        let err = IpcError::new("TEST_CODE", "Test message");
        assert_eq!(err.code, "TEST_CODE");
        assert_eq!(err.message, "Test message");
        assert!(err.details.is_none());
    }

    #[test]
    fn test_ipc_error_with_details() {
        let err = IpcError::new("TEST_CODE", "Test message").with_details("Additional details");
        assert_eq!(err.details, Some("Additional details".to_string()));
    }

    #[test]
    fn test_serial_params_default() {
        let params = SerialParams::default();
        assert_eq!(params.port, "");
        assert_eq!(params.baud_rate, 115200);
        assert_eq!(params.data_bits, DataBits::Eight);
        assert_eq!(params.parity, Parity::None);
        assert_eq!(params.stop_bits, StopBits::One);
        assert_eq!(params.flow_control, FlowControl::None);
    }

    #[test]
    fn test_telnet_params_default() {
        let params = TelnetParams::default();
        assert_eq!(params.host, "");
        assert_eq!(params.port, 23);
        assert_eq!(params.connect_timeout_secs, 10);
    }

    #[test]
    fn test_connection_stats_default() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
    }

    #[test]
    fn test_data_bits_variants() {
        assert_ne!(
            std::mem::discriminant(&DataBits::Seven),
            std::mem::discriminant(&DataBits::Eight)
        );
    }

    #[test]
    fn test_parity_variants() {
        let variants = vec![Parity::None, Parity::Odd, Parity::Even];
        for i in 0..variants.len() {
            for j in i + 1..variants.len() {
                assert_ne!(
                    std::mem::discriminant(&variants[i]),
                    std::mem::discriminant(&variants[j])
                );
            }
        }
    }

    #[test]
    fn test_stop_bits_variants() {
        assert_ne!(
            std::mem::discriminant(&StopBits::One),
            std::mem::discriminant(&StopBits::Two)
        );
    }

    #[test]
    fn test_flow_control_variants() {
        let variants = vec![
            FlowControl::None,
            FlowControl::Software,
            FlowControl::Hardware,
        ];
        for i in 0..variants.len() {
            for j in i + 1..variants.len() {
                assert_ne!(
                    std::mem::discriminant(&variants[i]),
                    std::mem::discriminant(&variants[j])
                );
            }
        }
    }

    #[test]
    fn test_connection_status_variants() {
        let variants = vec![
            ConnectionStatus::Disconnected,
            ConnectionStatus::Connecting,
            ConnectionStatus::Connected,
            ConnectionStatus::Error,
        ];
        for i in 0..variants.len() {
            for j in i + 1..variants.len() {
                assert_ne!(
                    std::mem::discriminant(&variants[i]),
                    std::mem::discriminant(&variants[j])
                );
            }
        }
    }

    #[test]
    fn test_log_direction_variants() {
        assert_ne!(
            std::mem::discriminant(&LogDirection::Input),
            std::mem::discriminant(&LogDirection::Output)
        );
    }
}
