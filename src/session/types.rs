//! Session type definitions
//!
//! Defines all session-related types: identifiers, configuration, statistics,
//! state enums, and metadata structures.

/// Session identifier type
pub type SessionId = String;

/// Session state enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session has been created but not yet started
    Created,
    /// Session is attempting to connect
    Connecting,
    /// Session is connected and active
    Connected,
    /// Session is in the process of disconnecting
    Disconnecting,
    /// Session has been disconnected
    Disconnected,
    /// Session encountered an error
    Error(String),
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Created => write!(f, "Created"),
            SessionState::Connecting => write!(f, "Connecting"),
            SessionState::Connected => write!(f, "Connected"),
            SessionState::Disconnecting => write!(f, "Disconnecting"),
            SessionState::Disconnected => write!(f, "Disconnected"),
            SessionState::Error(err) => write!(f, "Error: {}", err),
        }
    }
}

/// Session metadata
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// Session ID
    pub id: String,
    /// Session name
    pub name: String,
    /// Connection type (Serial, Telnet, etc.)
    pub connection_type: String,
    /// When the session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last active
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl SessionMetadata {
    /// Set the session name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

/// Session manager configuration
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Maximum number of concurrent sessions
    pub max_sessions: usize,
    /// Default name prefix for new sessions
    pub default_name_prefix: String,
    /// Whether to automatically cleanup expired sessions
    pub auto_cleanup: bool,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            default_name_prefix: "Session".to_string(),
            auto_cleanup: true,
            session_timeout_secs: 3600,
        }
    }
}

/// Session manager statistics
#[derive(Debug, Clone)]
pub struct SessionManagerStats {
    /// Total sessions created
    pub total_created: u64,
    /// Total sessions destroyed
    pub total_destroyed: u64,
    /// Currently active sessions
    pub active_sessions: usize,
    /// Peak concurrent sessions
    pub peak_concurrent: usize,
}

impl Default for SessionManagerStats {
    fn default() -> Self {
        Self {
            total_created: 0,
            total_destroyed: 0,
            active_sessions: 0,
            peak_concurrent: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_display() {
        assert_eq!(SessionState::Created.to_string(), "Created");
        assert_eq!(SessionState::Connecting.to_string(), "Connecting");
        assert_eq!(SessionState::Connected.to_string(), "Connected");
        assert_eq!(SessionState::Disconnecting.to_string(), "Disconnecting");
        assert_eq!(SessionState::Disconnected.to_string(), "Disconnected");
        assert_eq!(
            SessionState::Error("test error".to_string()).to_string(),
            "Error: test error"
        );
    }

    #[test]
    fn test_session_state_equality() {
        assert_eq!(SessionState::Created, SessionState::Created);
        assert_ne!(SessionState::Created, SessionState::Connected);
        assert_ne!(
            SessionState::Error("a".to_string()),
            SessionState::Error("b".to_string())
        );
    }

    #[test]
    fn test_session_manager_config_default() {
        let config = SessionManagerConfig::default();
        assert_eq!(config.max_sessions, 10);
        assert_eq!(config.default_name_prefix, "Session");
        assert!(config.auto_cleanup);
        assert_eq!(config.session_timeout_secs, 3600);
    }

    #[test]
    fn test_session_manager_stats_default() {
        let stats = SessionManagerStats::default();
        assert_eq!(stats.total_created, 0);
        assert_eq!(stats.total_destroyed, 0);
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.peak_concurrent, 0);
    }
}
