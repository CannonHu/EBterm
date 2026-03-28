//! Connection module for embedded debugger
//!
//! Provides connection factory and types for serial port and Telnet connections

use thiserror::Error;

pub mod types;
pub mod serial;
pub mod telnet;
pub mod discovery;

pub use self::types::{
    Connection, ConnectionConfig, ConnectionFactory, ConnectionStats, ConnectionStatus,
    ConnectionType, DataBits, FlowControl, Parity, SerialConfig, StopBits, TelnetConfig,
    ConnectionHandle,
};
pub use self::serial::SerialConnection;
pub use self::telnet::TelnetConnection;
pub use self::discovery::{discover_serial_ports, DiscoveredPort, DiscoveryError};

/// Connection-related errors
#[derive(Error, Debug)]
pub enum ConnectionError {
    /// Failed to open serial port
    #[error("Failed to open serial port '{port}': {reason}")]
    OpenFailed {
        /// The port name attempted
        port: String,
        /// The reason for failure
        reason: String,
    },

    /// Connection not connected
    #[error("Connection is not connected")]
    NotConnected,

    /// Failed to read from connection
    #[error("Read error on '{port}': {reason}")]
    ReadFailed {
        /// The port name
        port: String,
        /// The reason for failure
        reason: String,
    },

    /// Failed to write to connection
    #[error("Write error on '{port}': {reason}")]
    WriteFailed {
        /// The port name
        port: String,
        /// The reason for failure
        reason: String,
    },

    /// Connection timeout
    #[error("Connection timeout for '{port}' after {timeout_ms}ms")]
    Timeout {
        /// The port name
        port: String,
        /// Timeout in milliseconds
        timeout_ms: u64,
    },

    /// Invalid port name
    #[error("Invalid port name: {0}")]
    InvalidPort(String),

    /// Invalid baud rate
    #[error("Invalid baud rate: {0}")]
    InvalidBaudRate(u32),

    /// Connection already exists
    #[error("Connection already exists for port '{port}'")]
    AlreadyExists {
        /// The port name
        port: String,
    },

    /// Connection not found
    #[error("Connection not found for port '{port}'")]
    NotFound {
        /// The port name
        port: String,
    },

    /// Telnet-specific errors
    #[error("Telnet error: {0}")]
    Telnet(String),

    /// Serial-specific errors
    #[error("Serial error: {0}")]
    Serial(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic connection error
    #[error("{0}")]
    Generic(String),
}

impl ConnectionError {
    /// Get error code for IPC communication
    pub fn code(&self) -> &'static str {
        match self {
            ConnectionError::OpenFailed { .. } => "CONNECTION_OPEN_FAILED",
            ConnectionError::NotConnected => "CONNECTION_NOT_CONNECTED",
            ConnectionError::ReadFailed { .. } => "CONNECTION_READ_FAILED",
            ConnectionError::WriteFailed { .. } => "CONNECTION_WRITE_FAILED",
            ConnectionError::Timeout { .. } => "CONNECTION_TIMEOUT",
            ConnectionError::InvalidPort(_) => "CONNECTION_INVALID_PORT",
            ConnectionError::InvalidBaudRate(_) => "CONNECTION_INVALID_BAUD_RATE",
            ConnectionError::AlreadyExists { .. } => "CONNECTION_ALREADY_EXISTS",
            ConnectionError::NotFound { .. } => "CONNECTION_NOT_FOUND",
            ConnectionError::Telnet(_) => "CONNECTION_TELNET_ERROR",
            ConnectionError::Serial(_) => "CONNECTION_SERIAL_ERROR",
            ConnectionError::Io(_) => "CONNECTION_IO_ERROR",
            ConnectionError::Generic(_) => "CONNECTION_GENERIC_ERROR",
        }
    }

    /// Get the port name if available
    pub fn port(&self) -> Option<&str> {
        match self {
            ConnectionError::OpenFailed { port, .. } => Some(port),
            ConnectionError::ReadFailed { port, .. } => Some(port),
            ConnectionError::WriteFailed { port, .. } => Some(port),
            ConnectionError::Timeout { port, .. } => Some(port),
            ConnectionError::AlreadyExists { port } => Some(port),
            ConnectionError::NotFound { port } => Some(port),
            ConnectionError::InvalidPort(port) => Some(port),
            ConnectionError::NotConnected
            | ConnectionError::Telnet(_)
            | ConnectionError::Serial(_)
            | ConnectionError::Io(_)
            | ConnectionError::InvalidBaudRate(_)
            | ConnectionError::Generic(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_error_open_failed() {
        let err = ConnectionError::OpenFailed {
            port: "/dev/ttyUSB0".to_string(),
            reason: "Permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/dev/ttyUSB0"));
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("Failed to open"));
    }

    #[test]
    fn test_connection_error_not_connected() {
        let err = ConnectionError::NotConnected;
        let msg = err.to_string();
        assert!(msg.contains("not connected"));
        assert_eq!(err.code(), "CONNECTION_NOT_CONNECTED");
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_connection_error_read_failed() {
        let err = ConnectionError::ReadFailed {
            port: "/dev/ttyS0".to_string(),
            reason: "Input/output error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/dev/ttyS0"));
        assert!(msg.contains("Input/output error"));
    }

    #[test]
    fn test_connection_error_write_failed() {
        let err = ConnectionError::WriteFailed {
            port: "COM3".to_string(),
            reason: "Buffer overflow".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("COM3"));
        assert!(msg.contains("Buffer overflow"));
    }

    #[test]
    fn test_connection_error_timeout() {
        let err = ConnectionError::Timeout {
            port: "/dev/ttyACM0".to_string(),
            timeout_ms: 5000,
        };
        let msg = err.to_string();
        assert!(msg.contains("/dev/ttyACM0"));
        assert!(msg.contains("5000"));
    }

    #[test]
    fn test_connection_error_invalid_port() {
        let err = ConnectionError::InvalidPort("".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid port"));
    }

    #[test]
    fn test_connection_error_invalid_baud_rate() {
        let err = ConnectionError::InvalidBaudRate(0);
        let msg = err.to_string();
        assert!(msg.contains("Invalid baud rate"));
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_connection_error_already_exists() {
        let err = ConnectionError::AlreadyExists {
            port: "/dev/ttyUSB1".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/dev/ttyUSB1"));
        assert!(msg.contains("already exists"));
    }

    #[test]
    fn test_connection_error_not_found() {
        let err = ConnectionError::NotFound {
            port: "COM99".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("COM99"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_connection_error_telnet() {
        let err = ConnectionError::Telnet("Connection refused".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Telnet"));
        assert!(msg.contains("Connection refused"));
    }

    #[test]
    fn test_connection_error_serial() {
        let err = ConnectionError::Serial("Port busy".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Serial"));
        assert!(msg.contains("Port busy"));
        assert_eq!(err.code(), "CONNECTION_SERIAL_ERROR");
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_connection_error_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err = ConnectionError::from(io_err);
        let msg = err.to_string();
        assert!(msg.contains("I/O error"));
        assert_eq!(err.code(), "CONNECTION_IO_ERROR");
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_connection_error_generic() {
        let err = ConnectionError::Generic("Unknown error occurred".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown error occurred"));
    }

    #[test]
    fn test_error_code_open_failed() {
        let err = ConnectionError::OpenFailed {
            port: "test".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(err.code(), "CONNECTION_OPEN_FAILED");
    }

    #[test]
    fn test_error_code_read_failed() {
        let err = ConnectionError::ReadFailed {
            port: "test".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(err.code(), "CONNECTION_READ_FAILED");
    }

    #[test]
    fn test_error_code_write_failed() {
        let err = ConnectionError::WriteFailed {
            port: "test".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(err.code(), "CONNECTION_WRITE_FAILED");
    }

    #[test]
    fn test_error_code_timeout() {
        let err = ConnectionError::Timeout {
            port: "test".to_string(),
            timeout_ms: 100,
        };
        assert_eq!(err.code(), "CONNECTION_TIMEOUT");
    }

    #[test]
    fn test_error_code_invalid_port() {
        let err = ConnectionError::InvalidPort("test".to_string());
        assert_eq!(err.code(), "CONNECTION_INVALID_PORT");
    }

    #[test]
    fn test_error_code_invalid_baud_rate() {
        let err = ConnectionError::InvalidBaudRate(9600);
        assert_eq!(err.code(), "CONNECTION_INVALID_BAUD_RATE");
    }

    #[test]
    fn test_error_code_already_exists() {
        let err = ConnectionError::AlreadyExists {
            port: "test".to_string(),
        };
        assert_eq!(err.code(), "CONNECTION_ALREADY_EXISTS");
    }

    #[test]
    fn test_error_code_not_found() {
        let err = ConnectionError::NotFound {
            port: "test".to_string(),
        };
        assert_eq!(err.code(), "CONNECTION_NOT_FOUND");
    }

    #[test]
    fn test_error_code_telnet() {
        let err = ConnectionError::Telnet("test".to_string());
        assert_eq!(err.code(), "CONNECTION_TELNET_ERROR");
    }

    #[test]
    fn test_error_code_generic() {
        let err = ConnectionError::Generic("test".to_string());
        assert_eq!(err.code(), "CONNECTION_GENERIC_ERROR");
    }

    #[test]
    fn test_port_extraction_open_failed() {
        let err = ConnectionError::OpenFailed {
            port: "/dev/ttyUSB0".to_string(),
            reason: "error".to_string(),
        };
        assert_eq!(err.port(), Some("/dev/ttyUSB0"));
    }

    #[test]
    fn test_port_extraction_read_failed() {
        let err = ConnectionError::ReadFailed {
            port: "/dev/ttyS0".to_string(),
            reason: "error".to_string(),
        };
        assert_eq!(err.port(), Some("/dev/ttyS0"));
    }

    #[test]
    fn test_port_extraction_write_failed() {
        let err = ConnectionError::WriteFailed {
            port: "COM3".to_string(),
            reason: "error".to_string(),
        };
        assert_eq!(err.port(), Some("COM3"));
    }

    #[test]
    fn test_port_extraction_timeout() {
        let err = ConnectionError::Timeout {
            port: "/dev/ttyACM0".to_string(),
            timeout_ms: 1000,
        };
        assert_eq!(err.port(), Some("/dev/ttyACM0"));
    }

    #[test]
    fn test_port_extraction_invalid_port() {
        let err = ConnectionError::InvalidPort("invalid".to_string());
        assert_eq!(err.port(), Some("invalid"));
    }

    #[test]
    fn test_port_extraction_already_exists() {
        let err = ConnectionError::AlreadyExists {
            port: "test".to_string(),
        };
        assert_eq!(err.port(), Some("test"));
    }

    #[test]
    fn test_port_extraction_not_found() {
        let err = ConnectionError::NotFound {
            port: "test".to_string(),
        };
        assert_eq!(err.port(), Some("test"));
    }

    #[test]
    fn test_port_extraction_invalid_baud_rate() {
        let err = ConnectionError::InvalidBaudRate(9600);
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_port_extraction_telnet() {
        let err = ConnectionError::Telnet("error".to_string());
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_port_extraction_generic() {
        let err = ConnectionError::Generic("error".to_string());
        assert_eq!(err.port(), None);
    }

    #[test]
    fn test_empty_port_name() {
        let err = ConnectionError::InvalidPort("".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid port"));
        assert_eq!(err.port(), Some(""));
    }

    #[test]
    fn test_empty_error_message() {
        let err = ConnectionError::Generic("".to_string());
        let msg = err.to_string();
        assert!(msg.is_empty() || msg.contains("Generic"));
    }

    #[test]
    fn test_special_characters_in_port() {
        let err = ConnectionError::OpenFailed {
            port: "/dev/ttyUSB-1_2".to_string(),
            reason: "Error: cannot access".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/dev/ttyUSB-1_2"));
    }

    #[test]
    fn test_very_long_port_name() {
        let long_port = "a".repeat(1000);
        let err = ConnectionError::InvalidPort(long_port.clone());
        assert_eq!(err.port(), Some(long_port.as_str()));
    }

    #[test]
    fn test_baud_rate_zero() {
        let err = ConnectionError::InvalidBaudRate(0);
        let msg = err.to_string();
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_baud_rate_max_u32() {
        let err = ConnectionError::InvalidBaudRate(u32::MAX);
        let msg = err.to_string();
        assert!(msg.contains("4294967295") || msg.contains(&u32::MAX.to_string()));
    }

    #[test]
    fn test_timeout_edge_cases() {
        let err = ConnectionError::Timeout {
            port: "test".to_string(),
            timeout_ms: 0,
        };
        let msg = err.to_string();
        assert!(msg.contains("0"));

        let err = ConnectionError::Timeout {
            port: "test".to_string(),
            timeout_ms: u64::MAX,
        };
        let msg = err.to_string();
        assert!(msg.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn test_debug_format() {
        let err = ConnectionError::OpenFailed {
            port: "test".to_string(),
            reason: "error".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("OpenFailed"));
        assert!(debug.contains("test"));
    }
}
