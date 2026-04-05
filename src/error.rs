//! Error types for the embedded debugger
//!
//! Centralized error handling with detailed error codes and messages

use thiserror::Error;

/// Common error type for the embedded debugger
#[derive(Error, Debug)]
pub enum Error {
    /// Connection-related errors
    #[error("Connection error: {0}")]
    Connection(#[from] crate::connection::ConnectionError),

    /// Logger-related errors
    #[error("Logger error: {0}")]
    Logger(#[from] crate::logger::LoggerError),


    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic errors
    #[error("{0}")]
    Generic(String),
}

impl Error {
    /// Get error code for IPC communication
    pub fn code(&self) -> String {
        match self {
            Error::Connection(_) => "CONNECTION_ERROR".to_string(),
            Error::Logger(_) => "LOGGER_ERROR".to_string(),
            Error::Io(_) => "IO_ERROR".to_string(),
            Error::Serialization(_) => "SERIALIZATION_ERROR".to_string(),
            Error::Generic(_) => "GENERIC_ERROR".to_string(),
        }
    }

    /// Get detailed error message (if available)
    pub fn details(&self) -> Option<String> {
        None
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, Error>;
