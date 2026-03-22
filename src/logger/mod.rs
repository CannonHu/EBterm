//! Logger module for embedded debugger
//!
//! Manages logging of connection data to files

use thiserror::Error;

mod file;
mod traits;
pub use self::traits::{LogDirection, LogEntry, Logger, LoggerConfig, LoggerStats};
pub use self::file::FileLogger;

/// Logger-related errors
#[derive(Error, Debug)]
pub enum LoggerError {
    /// Failed to open log file
    #[error("Failed to open log file '{path}': {reason}")]
    OpenFailed {
        /// File path
        path: String,
        /// Reason for failure
        reason: String,
    },

    /// Failed to write to log file
    #[error("Failed to write to log file '{path}': {reason}")]
    WriteFailed {
        /// File path
        path: String,
        /// Reason for failure
        reason: String,
    },

    /// Failed to close log file
    #[error("Failed to close log file '{path}': {reason}")]
    CloseFailed {
        /// File path
        path: String,
        /// Reason for failure
        reason: String,
    },

    /// Log file not open
    #[error("Log file '{path}' is not open")]
    NotOpen {
        /// File path
        path: String,
    },

    /// Log file already open
    #[error("Log file '{path}' is already open")]
    AlreadyOpen {
        /// File path
        path: String,
    },

    /// Invalid log file path
    #[error("Invalid log file path: {0}")]
    InvalidPath(String),

    /// Disk full
    #[error("Disk full while writing to '{path}'")]
    DiskFull {
        /// File path
        path: String,
    },

    /// File too large
    #[error("Log file '{path}' too large: {size} bytes exceeds limit {limit}")]
    FileTooLarge {
        /// File path
        path: String,
        /// Current size
        size: u64,
        /// Size limit
        limit: u64,
    },

    /// Logging not enabled
    #[error("Logging is not enabled for session '{session_id}'")]
    NotEnabled {
        /// Session ID
        session_id: String,
    },

    /// Generic logger error
    #[error("{0}")]
    Generic(String),
}

impl LoggerError {
    /// Get error code for IPC communication
    pub fn code(&self) -> &'static str {
        match self {
            LoggerError::OpenFailed { .. } => "LOGGER_OPEN_FAILED",
            LoggerError::WriteFailed { .. } => "LOGGER_WRITE_FAILED",
            LoggerError::CloseFailed { .. } => "LOGGER_CLOSE_FAILED",
            LoggerError::NotOpen { .. } => "LOGGER_NOT_OPEN",
            LoggerError::AlreadyOpen { .. } => "LOGGER_ALREADY_OPEN",
            LoggerError::InvalidPath(_) => "LOGGER_INVALID_PATH",
            LoggerError::DiskFull { .. } => "LOGGER_DISK_FULL",
            LoggerError::FileTooLarge { .. } => "LOGGER_FILE_TOO_LARGE",
            LoggerError::NotEnabled { .. } => "LOGGER_NOT_ENABLED",
            LoggerError::Generic(_) => "LOGGER_GENERIC_ERROR",
        }
    }

    /// Get file path if available
    pub fn path(&self) -> Option<&str> {
        match self {
            LoggerError::OpenFailed { path, .. } => Some(path),
            LoggerError::WriteFailed { path, .. } => Some(path),
            LoggerError::CloseFailed { path, .. } => Some(path),
            LoggerError::NotOpen { path } => Some(path),
            LoggerError::AlreadyOpen { path } => Some(path),
            LoggerError::DiskFull { path } => Some(path),
            LoggerError::FileTooLarge { path, .. } => Some(path),
            LoggerError::InvalidPath(path) => Some(path),
            LoggerError::NotEnabled { .. } | LoggerError::Generic(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_error_open_failed() {
        let err = LoggerError::OpenFailed {
            path: "/tmp/log.txt".to_string(),
            reason: "Permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("Failed to open"));
        assert_eq!(err.code(), "LOGGER_OPEN_FAILED");
        assert_eq!(err.path(), Some("/tmp/log.txt"));
    }

    #[test]
    fn test_logger_error_write_failed() {
        let err = LoggerError::WriteFailed {
            path: "/tmp/log.txt".to_string(),
            reason: "Disk error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("Disk error"));
        assert_eq!(err.code(), "LOGGER_WRITE_FAILED");
    }

    #[test]
    fn test_logger_error_close_failed() {
        let err = LoggerError::CloseFailed {
            path: "/tmp/log.txt".to_string(),
            reason: "Buffer flush failed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("Buffer flush failed"));
        assert_eq!(err.code(), "LOGGER_CLOSE_FAILED");
    }

    #[test]
    fn test_logger_error_not_open() {
        let err = LoggerError::NotOpen {
            path: "/tmp/log.txt".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("not open"));
        assert_eq!(err.code(), "LOGGER_NOT_OPEN");
    }

    #[test]
    fn test_logger_error_already_open() {
        let err = LoggerError::AlreadyOpen {
            path: "/tmp/log.txt".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("already open"));
        assert_eq!(err.code(), "LOGGER_ALREADY_OPEN");
    }

    #[test]
    fn test_logger_error_invalid_path() {
        let err = LoggerError::InvalidPath("/invalid/path".to_string());
        let msg = err.to_string();
        assert!(msg.contains("/invalid/path"));
        assert!(msg.contains("Invalid log file path"));
        assert_eq!(err.code(), "LOGGER_INVALID_PATH");
    }

    #[test]
    fn test_logger_error_disk_full() {
        let err = LoggerError::DiskFull {
            path: "/tmp/log.txt".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("Disk full"));
        assert_eq!(err.code(), "LOGGER_DISK_FULL");
    }

    #[test]
    fn test_logger_error_file_too_large() {
        let err = LoggerError::FileTooLarge {
            path: "/tmp/log.txt".to_string(),
            size: 1024 * 1024 * 100,
            limit: 1024 * 1024 * 10,
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log.txt"));
        assert!(msg.contains("104857600")); // 100MB
        assert!(msg.contains("10485760")); // 10MB
        assert_eq!(err.code(), "LOGGER_FILE_TOO_LARGE");
    }

    #[test]
    fn test_logger_error_not_enabled() {
        let err = LoggerError::NotEnabled {
            session_id: "session-123".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-123"));
        assert!(msg.contains("not enabled"));
        assert_eq!(err.code(), "LOGGER_NOT_ENABLED");
    }

    #[test]
    fn test_logger_error_generic() {
        let err = LoggerError::Generic("Unknown error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown error"));
        assert_eq!(err.code(), "LOGGER_GENERIC_ERROR");
    }

    #[test]
    fn test_empty_path() {
        let err = LoggerError::InvalidPath("".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid log file path"));
        assert_eq!(err.path(), Some(""));
    }

    #[test]
    fn test_empty_error_message() {
        let err = LoggerError::Generic("".to_string());
        let msg = err.to_string();
        assert!(msg.is_empty() || msg.contains("Generic"));
    }

    #[test]
    fn test_special_characters_in_path() {
        let err = LoggerError::OpenFailed {
            path: "/tmp/log-123_abc.txt".to_string(),
            reason: "Error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/log-123_abc.txt"));
    }

    #[test]
    fn test_very_long_path() {
        let long_path = "/tmp/".to_string() + &"a".repeat(1000);
        let err = LoggerError::InvalidPath(long_path.clone());
        assert_eq!(err.path(), Some(long_path.as_str()));
    }

    #[test]
    fn test_file_size_edge_cases() {
        // Zero size
        let err = LoggerError::FileTooLarge {
            path: "/tmp/log.txt".to_string(),
            size: 0,
            limit: 1024,
        };
        let msg = err.to_string();
        assert!(msg.contains("0"));

        // Max size
        let err = LoggerError::FileTooLarge {
            path: "/tmp/log.txt".to_string(),
            size: u64::MAX,
            limit: 1024,
        };
        let msg = err.to_string();
        assert!(msg.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn test_debug_format() {
        let err = LoggerError::OpenFailed {
            path: "test".to_string(),
            reason: "error".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("OpenFailed"));
        assert!(debug.contains("test"));
    }
}
