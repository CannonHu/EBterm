//! Command module for embedded debugger
//!
//! Handles command file parsing and execution

use thiserror::Error;

pub mod parser;
pub mod manager;

pub use parser::{CommandParser, DefaultCommandParser, ParsedCommand};
pub use manager::{CommandManager, DefaultCommandManager, ExecutionResult};

/// Command-related errors
#[derive(Error, Debug)]
pub enum CommandError {
    /// Failed to parse command file
    #[error("Failed to parse command file '{path}': {reason}")]
    ParseFailed {
        /// File path
        path: String,
        /// Reason for failure
        reason: String,
    },

    /// Failed to read command file
    #[error("Failed to read command file '{path}': {reason}")]
    ReadFailed {
        /// File path
        path: String,
        /// Reason for failure
        reason: String,
    },

    /// Invalid command file format
    #[error("Invalid command file format '{path}': {detail}")]
    InvalidFormat {
        /// File path
        path: String,
        /// Detailed error information
        detail: String,
    },

    /// Invalid command syntax
    #[error("Invalid command syntax at line {line}: {detail}")]
    InvalidSyntax {
        /// Line number where error occurred
        line: usize,
        /// Detailed error information
        detail: String,
    },

    /// Command file not found
    #[error("Command file not found: {path}")]
    FileNotFound {
        /// File path
        path: String,
    },

    /// Command file is empty
    #[error("Command file is empty: {path}")]
    EmptyFile {
        /// File path
        path: String,
    },

    /// Command file too large
    #[error("Command file '{path}' too large: {size} bytes exceeds limit {limit}")]
    TooLarge {
        /// File path
        path: String,
        /// Current size
        size: u64,
        /// Size limit
        limit: u64,
    },

    /// Generic command error
    #[error("{0}")]
    Generic(String),
}

impl CommandError {
    /// Get error code for IPC communication
    pub fn code(&self) -> &'static str {
        match self {
            CommandError::ParseFailed { .. } => "COMMAND_PARSE_FAILED",
            CommandError::ReadFailed { .. } => "COMMAND_READ_FAILED",
            CommandError::InvalidFormat { .. } => "COMMAND_INVALID_FORMAT",
            CommandError::InvalidSyntax { .. } => "COMMAND_INVALID_SYNTAX",
            CommandError::FileNotFound { .. } => "COMMAND_FILE_NOT_FOUND",
            CommandError::EmptyFile { .. } => "COMMAND_EMPTY_FILE",
            CommandError::TooLarge { .. } => "COMMAND_TOO_LARGE",
            CommandError::Generic(_) => "COMMAND_GENERIC_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic functionality tests

    #[test]
    fn test_command_error_parse_failed() {
        let err = CommandError::ParseFailed {
            path: "/commands/test.cmd".to_string(),
            reason: "Unexpected token".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/test.cmd"));
        assert!(msg.contains("Unexpected token"));
        assert!(msg.contains("Failed to parse"));
        assert_eq!(err.code(), "COMMAND_PARSE_FAILED");
    }

    #[test]
    fn test_command_error_read_failed() {
        let err = CommandError::ReadFailed {
            path: "/commands/script.cmd".to_string(),
            reason: "Permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/script.cmd"));
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("Failed to read"));
        assert_eq!(err.code(), "COMMAND_READ_FAILED");
    }

    #[test]
    fn test_command_error_invalid_format() {
        let err = CommandError::InvalidFormat {
            path: "/commands/data.txt".to_string(),
            detail: "Expected JSON format".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/data.txt"));
        assert!(msg.contains("Expected JSON format"));
        assert!(msg.contains("Invalid"));
        assert_eq!(err.code(), "COMMAND_INVALID_FORMAT");
    }

    #[test]
    fn test_command_error_invalid_syntax() {
        let err = CommandError::InvalidSyntax {
            line: 42,
            detail: "Unknown command 'xyz'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("Unknown command"));
        assert!(msg.contains("Invalid"));
        assert_eq!(err.code(), "COMMAND_INVALID_SYNTAX");
    }

    #[test]
    fn test_command_error_file_not_found() {
        let err = CommandError::FileNotFound {
            path: "/commands/missing.cmd".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/missing.cmd"));
        assert!(msg.contains("not found"));
        assert_eq!(err.code(), "COMMAND_FILE_NOT_FOUND");
    }

    #[test]
    fn test_command_error_empty_file() {
        let err = CommandError::EmptyFile {
            path: "/commands/empty.cmd".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/empty.cmd"));
        assert!(msg.contains("empty"));
        assert_eq!(err.code(), "COMMAND_EMPTY_FILE");
    }

    #[test]
    fn test_command_error_too_large() {
        let err = CommandError::TooLarge {
            path: "/commands/big.cmd".to_string(),
            size: 1024 * 1024 * 50,
            limit: 1024 * 1024 * 10,
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/big.cmd"));
        assert!(msg.contains("52428800")); // 50MB
        assert!(msg.contains("10485760")); // 10MB
        assert!(msg.contains("too large"));
        assert_eq!(err.code(), "COMMAND_TOO_LARGE");
    }

    #[test]
    fn test_command_error_generic() {
        let err = CommandError::Generic("Unknown command error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown command error"));
        assert_eq!(err.code(), "COMMAND_GENERIC_ERROR");
    }

    // Input validation tests

    #[test]
    fn test_empty_path() {
        let err = CommandError::FileNotFound {
            path: "".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_empty_error_message() {
        let err = CommandError::Generic("".to_string());
        let msg = err.to_string();
        assert!(msg.is_empty() || msg.contains("Generic"));
    }

    #[test]
    fn test_special_characters_in_path() {
        let err = CommandError::ParseFailed {
            path: "/commands/test-123_abc.cmd".to_string(),
            reason: "Error: invalid syntax".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/commands/test-123_abc.cmd"));
    }

    #[test]
    fn test_very_long_path() {
        let long_path = "/commands/".to_string() + &"a".repeat(1000);
        let err = CommandError::FileNotFound {
            path: long_path.clone(),
        };
        let msg = err.to_string();
        assert!(msg.contains(&long_path));
    }

    #[test]
    fn test_special_characters_in_reason() {
        let err = CommandError::ParseFailed {
            path: "test.cmd".to_string(),
            reason: "Error @#$% invalid <token>".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Error @#$% invalid <token>"));
    }

    #[test]
    fn test_special_characters_in_detail() {
        let err = CommandError::InvalidFormat {
            path: "test.txt".to_string(),
            detail: "Expected: 'value' but got [null]".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Expected: 'value' but got [null]"));
    }

    // Edge case tests

    #[test]
    fn test_file_size_zero() {
        let err = CommandError::TooLarge {
            path: "test.cmd".to_string(),
            size: 0,
            limit: 1024,
        };
        let msg = err.to_string();
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_file_size_max() {
        let err = CommandError::TooLarge {
            path: "test.cmd".to_string(),
            size: u64::MAX,
            limit: 1024,
        };
        let msg = err.to_string();
        assert!(msg.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn test_line_number_zero() {
        let err = CommandError::InvalidSyntax {
            line: 0,
            detail: "Empty command".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("0"));
        assert!(msg.contains("Empty command"));
    }

    #[test]
    fn test_line_number_max() {
        let err = CommandError::InvalidSyntax {
            line: usize::MAX,
            detail: "Error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains(&usize::MAX.to_string()));
    }

    #[test]
    fn test_limit_zero() {
        let err = CommandError::TooLarge {
            path: "test.cmd".to_string(),
            size: 100,
            limit: 0,
        };
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_limit_max() {
        let err = CommandError::TooLarge {
            path: "test.cmd".to_string(),
            size: 100,
            limit: u64::MAX,
        };
        let msg = err.to_string();
        assert!(msg.contains(&u64::MAX.to_string()));
    }


    // Debug format test

    #[test]
    fn test_debug_format() {
        let err = CommandError::ParseFailed {
            path: "test.cmd".to_string(),
            reason: "error".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("ParseFailed"));
        assert!(debug.contains("test.cmd"));
    }
}
