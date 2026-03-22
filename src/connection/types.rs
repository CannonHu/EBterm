//! Connection types and trait definitions
//!
//! Defines the core Connection trait and all associated types for the connection module.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ConnectionError;

/// Unique identifier for a connection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId(String);

impl ConnectionId {
    /// Create a new ConnectionId
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from existing string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Get inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a connection that can be shared across async contexts
pub type ConnectionHandle = Arc<Mutex<Box<dyn Connection>>>;

/// Core connection trait
#[async_trait]
pub trait Connection: Send + Sync {
    /// Get connection type
    fn connection_type(&self) -> ConnectionType;

    /// Get connection status
    fn status(&self) -> ConnectionStatus;

    /// Connect to the target
    async fn connect(&mut self) -> Result<(), ConnectionError>;

    /// Disconnect from the target
    async fn disconnect(&mut self) -> Result<(), ConnectionError>;

    /// Read data from the connection
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError>;

    /// Write data to the connection
    async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError>;

    /// Flush the connection
    async fn flush(&mut self) -> Result<(), ConnectionError>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Get connection statistics
    fn stats(&self) -> ConnectionStats;

    /// Clear connection statistics
    fn clear_stats(&mut self);
}

/// Connection type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Serial,
    Telnet,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionType::Serial => write!(f, "Serial"),
            ConnectionType::Telnet => write!(f, "Telnet"),
        }
    }
}

/// Connection status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Error => write!(f, "Error"),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub connected_at: Option<std::time::Instant>,
}

/// Connection configuration
#[derive(Debug, Clone)]
pub enum ConnectionConfig {
    Serial(SerialConfig),
    Telnet(TelnetConfig),
}

/// Serial connection configuration
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
}

impl Default for SerialConfig {
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

/// Telnet connection configuration
#[derive(Debug, Clone)]
pub struct TelnetConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout_secs: u64,
}

impl Default for TelnetConfig {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: 23,
            connect_timeout_secs: 30,
        }
    }
}

/// Data bits enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Seven,
    Eight,
}

/// Parity enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
}

/// Stop bits enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

/// Flow control enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

/// Factory for creating connection instances
pub struct ConnectionFactory;

impl ConnectionFactory {
    /// Create a new serial connection
    pub fn create_serial(config: SerialConfig) -> Box<dyn Connection> {
        use crate::connection::serial::SerialConnection;
        Box::new(SerialConnection::new(config))
    }

    /// Create a new Telnet connection
    pub fn create_telnet(config: TelnetConfig) -> Box<dyn Connection> {
        use crate::connection::telnet::TelnetConnection;
        Box::new(TelnetConnection::new(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Mock connection for trait testing ---

    struct MockConnection {
        status: ConnectionStatus,
        stats: ConnectionStats,
    }

    impl MockConnection {
        fn new() -> Self {
            Self {
                status: ConnectionStatus::Disconnected,
                stats: ConnectionStats::default(),
            }
        }
    }

    #[async_trait]
    impl Connection for MockConnection {
        fn connection_type(&self) -> ConnectionType {
            ConnectionType::Serial
        }

        fn status(&self) -> ConnectionStatus {
            self.status
        }

        async fn connect(&mut self) -> Result<(), ConnectionError> {
            self.status = ConnectionStatus::Connected;
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<(), ConnectionError> {
            self.status = ConnectionStatus::Disconnected;
            Ok(())
        }

        async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ConnectionError> {
            Ok(0)
        }

        async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
            self.stats.bytes_sent += buf.len() as u64;
            self.stats.packets_sent += 1;
            Ok(buf.len())
        }

        async fn flush(&mut self) -> Result<(), ConnectionError> {
            Ok(())
        }

        fn is_connected(&self) -> bool {
            matches!(self.status, ConnectionStatus::Connected)
        }

        fn stats(&self) -> ConnectionStats {
            self.stats.clone()
        }

        fn clear_stats(&mut self) {
            self.stats = ConnectionStats::default();
        }
    }

    // --- Connection trait tests ---

    #[tokio::test]
    async fn test_mock_connection_connect() {
        let mut conn = MockConnection::new();
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());

        conn.connect().await.unwrap();
        assert_eq!(conn.status(), ConnectionStatus::Connected);
        assert!(conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_disconnect() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        assert!(conn.is_connected());

        conn.disconnect().await.unwrap();
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_write() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        let data = b"Hello, World!";
        let written = conn.write(data).await.unwrap();
        assert_eq!(written, data.len());

        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, data.len() as u64);
        assert_eq!(stats.packets_sent, 1);
    }

    #[tokio::test]
    async fn test_mock_connection_read() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        let mut buf = [0u8; 1024];
        let read = conn.read(&mut buf).await.unwrap();
        assert_eq!(read, 0);
    }

    #[tokio::test]
    async fn test_mock_connection_flush() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        conn.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_connection_stats() {
        let mut conn = MockConnection::new();

        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert!(stats.connected_at.is_none());
    }

    #[tokio::test]
    async fn test_mock_connection_clear_stats() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        conn.write(b"test data").await.unwrap();
        let stats = conn.stats();
        assert!(stats.bytes_sent > 0);

        conn.clear_stats();
        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
    }

    // --- Display tests ---

    #[test]
    fn test_connection_type_display() {
        assert_eq!(ConnectionType::Serial.to_string(), "Serial");
        assert_eq!(ConnectionType::Telnet.to_string(), "Telnet");
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "Disconnected");
        assert_eq!(ConnectionStatus::Connecting.to_string(), "Connecting");
        assert_eq!(ConnectionStatus::Connected.to_string(), "Connected");
        assert_eq!(ConnectionStatus::Error.to_string(), "Error");
    }

    // --- Config tests ---

    #[test]
    fn test_serial_config_default() {
        let config = SerialConfig::default();
        assert_eq!(config.port, "");
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
        assert_eq!(config.flow_control, FlowControl::None);
    }

    #[test]
    fn test_telnet_config_default() {
        let config = TelnetConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 23);
        assert_eq!(config.connect_timeout_secs, 30);
    }

    #[test]
    fn test_connection_stats_default() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert!(stats.connected_at.is_none());
    }

    #[test]
    fn test_data_bits_variants() {
        let seven = DataBits::Seven;
        let eight = DataBits::Eight;
        assert_ne!(std::mem::discriminant(&seven), std::mem::discriminant(&eight));
    }

    #[test]
    fn test_parity_variants() {
        let none = Parity::None;
        let odd = Parity::Odd;
        let even = Parity::Even;
        assert_ne!(std::mem::discriminant(&none), std::mem::discriminant(&odd));
        assert_ne!(std::mem::discriminant(&odd), std::mem::discriminant(&even));
    }

    #[test]
    fn test_stop_bits_variants() {
        let one = StopBits::One;
        let two = StopBits::Two;
        assert_ne!(std::mem::discriminant(&one), std::mem::discriminant(&two));
    }

    #[test]
    fn test_flow_control_variants() {
        let none = FlowControl::None;
        let software = FlowControl::Software;
        let hardware = FlowControl::Hardware;
        assert_ne!(std::mem::discriminant(&none), std::mem::discriminant(&software));
        assert_ne!(std::mem::discriminant(&software), std::mem::discriminant(&hardware));
    }
}
