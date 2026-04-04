//! Logger module for embedded debugger
//!
//! Provides simple file-based logging that records plain text content.

use thiserror::Error;

mod file;
pub use self::file::FileLogger;

/// Logger-related errors
#[derive(Error, Debug)]
pub enum LoggerError {
    /// Failed to open log file
    #[error("Failed to open log file: {0}")]
    OpenFailed(String),

    /// Logger not started
    #[error("Logger not started")]
    NotStarted,

    /// Failed to write to log file
    #[error("Failed to write to log file: {0}")]
    WriteFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_error_open_failed() {
        let err = LoggerError::OpenFailed("/tmp/log.txt".to_string());
        assert!(err.to_string().contains("Failed to open"));
        assert!(err.to_string().contains("/tmp/log.txt"));
    }

    #[test]
    fn test_logger_error_not_started() {
        let err = LoggerError::NotStarted;
        assert!(err.to_string().contains("not started"));
    }

    #[test]
    fn test_logger_error_write_failed() {
        let err = LoggerError::WriteFailed("Disk error".to_string());
        assert!(err.to_string().contains("Failed to write"));
        assert!(err.to_string().contains("Disk error"));
    }
}
