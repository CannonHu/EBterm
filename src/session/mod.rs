//! Session module for embedded debugger
//!
//! Manages session lifecycle and connection pooling

pub use self::types::{SessionState, SessionMetadata, SessionId, SessionManagerConfig, SessionManagerStats};
pub use self::manager::SessionManager;
pub use self::connection_registry::ConnectionRegistry;

pub mod types;
pub mod manager;
pub mod connection_registry;

use thiserror::Error;

/// Session-related errors
#[derive(Error, Debug)]
pub enum SessionError {
    /// Session not found
    #[error("Session not found: {id}")]
    NotFound {
        /// Session ID
        id: String,
    },

    /// Session already exists
    #[error("Session already exists: {id}")]
    AlreadyExists {
        /// Session ID
        id: String,
    },

    /// Session is not connected
    #[error("Session '{id}' is not connected")]
    NotConnected {
        /// Session ID
        id: String,
    },

    /// Session is in invalid state
    #[error("Session '{id}' is in invalid state: {state}")]
    InvalidState {
        /// Session ID
        id: String,
        /// Current state
        state: String,
    },

    /// Failed to create session
    #[error("Failed to create session: {reason}")]
    CreationFailed {
        /// Reason for failure
        reason: String,
    },

    /// Failed to destroy session
    #[error("Failed to destroy session '{id}': {reason}")]
    DestructionFailed {
        /// Session ID
        id: String,
        /// Reason for failure
        reason: String,
    },

    /// Maximum number of sessions reached
    #[error("Maximum number of sessions ({max}) reached")]
    MaxSessionsReached {
        /// Maximum allowed sessions
        max: usize,
    },

    /// Generic session error
    #[error("{0}")]
    Generic(String),
}

impl SessionError {
    /// Get error code for IPC communication
    pub fn code(&self) -> &'static str {
        match self {
            SessionError::NotFound { .. } => "SESSION_NOT_FOUND",
            SessionError::AlreadyExists { .. } => "SESSION_ALREADY_EXISTS",
            SessionError::NotConnected { .. } => "SESSION_NOT_CONNECTED",
            SessionError::InvalidState { .. } => "SESSION_INVALID_STATE",
            SessionError::CreationFailed { .. } => "SESSION_CREATION_FAILED",
            SessionError::DestructionFailed { .. } => "SESSION_DESTRUCTION_FAILED",
            SessionError::MaxSessionsReached { .. } => "SESSION_MAX_REACHED",
            SessionError::Generic(_) => "SESSION_GENERIC_ERROR",
        }
    }

    /// Get session ID if available
    pub fn session_id(&self) -> Option<&str> {
        match self {
            SessionError::NotFound { id } => Some(id),
            SessionError::AlreadyExists { id } => Some(id),
            SessionError::NotConnected { id } => Some(id),
            SessionError::InvalidState { id, .. } => Some(id),
            SessionError::DestructionFailed { id, .. } => Some(id),
            SessionError::CreationFailed { .. }
            | SessionError::MaxSessionsReached { .. }
            | SessionError::Generic(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_error_not_found() {
        let err = SessionError::NotFound {
            id: "session-123".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-123"));
        assert!(msg.contains("not found"));
        assert_eq!(err.code(), "SESSION_NOT_FOUND");
        assert_eq!(err.session_id(), Some("session-123"));
    }

    #[test]
    fn test_session_error_already_exists() {
        let err = SessionError::AlreadyExists {
            id: "session-456".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-456"));
        assert!(msg.contains("already exists"));
        assert_eq!(err.code(), "SESSION_ALREADY_EXISTS");
    }

    #[test]
    fn test_session_error_not_connected() {
        let err = SessionError::NotConnected {
            id: "session-789".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-789"));
        assert!(msg.contains("not connected"));
        assert_eq!(err.code(), "SESSION_NOT_CONNECTED");
    }

    #[test]
    fn test_session_error_invalid_state() {
        let err = SessionError::InvalidState {
            id: "session-abc".to_string(),
            state: "DISCONNECTED".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-abc"));
        assert!(msg.contains("DISCONNECTED"));
        assert!(msg.contains("invalid state"));
        assert_eq!(err.code(), "SESSION_INVALID_STATE");
    }

    #[test]
    fn test_session_error_creation_failed() {
        let err = SessionError::CreationFailed {
            reason: "Connection refused".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Connection refused"));
        assert!(msg.contains("Failed to create"));
        assert_eq!(err.code(), "SESSION_CREATION_FAILED");
        assert_eq!(err.session_id(), None);
    }

    #[test]
    fn test_session_error_destruction_failed() {
        let err = SessionError::DestructionFailed {
            id: "session-xyz".to_string(),
            reason: "Resource busy".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-xyz"));
        assert!(msg.contains("Resource busy"));
        assert!(msg.contains("Failed to destroy"));
        assert_eq!(err.code(), "SESSION_DESTRUCTION_FAILED");
    }

    #[test]
    fn test_session_error_max_sessions_reached() {
        let err = SessionError::MaxSessionsReached { max: 10 };
        let msg = err.to_string();
        assert!(msg.contains("10"));
        assert!(msg.contains("Maximum number of sessions"));
        assert_eq!(err.code(), "SESSION_MAX_REACHED");
    }

    #[test]
    fn test_session_error_generic() {
        let err = SessionError::Generic("Unknown error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown error"));
        assert_eq!(err.code(), "SESSION_GENERIC_ERROR");
    }

    #[test]
    fn test_empty_session_id() {
        let err = SessionError::NotFound { id: "".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("not found"));
        assert_eq!(err.session_id(), Some(""));
    }

    #[test]
    fn test_empty_error_message() {
        let err = SessionError::Generic("".to_string());
        let msg = err.to_string();
        assert!(msg.is_empty() || msg.contains("Generic"));
    }

    #[test]
    fn test_special_characters_in_session_id() {
        let err = SessionError::NotFound {
            id: "session-123_ABC.def".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("session-123_ABC.def"));
    }

    #[test]
    fn test_very_long_session_id() {
        let long_id = "a".repeat(1000);
        let err = SessionError::NotFound {
            id: long_id.clone(),
        };
        assert_eq!(err.session_id(), Some(long_id.as_str()));
    }

    #[test]
    fn test_max_sessions_zero() {
        let err = SessionError::MaxSessionsReached { max: 0 };
        let msg = err.to_string();
        assert!(msg.contains("0"));
    }

    #[test]
    fn test_max_sessions_usize_max() {
        let err = SessionError::MaxSessionsReached { max: usize::MAX };
        let msg = err.to_string();
        assert!(msg.contains(&usize::MAX.to_string()));
    }

    #[test]
    fn test_empty_state() {
        let err = SessionError::InvalidState {
            id: "test".to_string(),
            state: "".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("test"));
        assert!(msg.contains("invalid state"));
    }

    #[test]
    fn test_debug_format() {
        let err = SessionError::NotFound {
            id: "test".to_string(),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
        assert!(debug.contains("test"));
    }
}
