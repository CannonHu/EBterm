//! Session manager implementation
//!
//! Manages the lifecycle of debugging sessions

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use super::types::{SessionId, SessionManagerConfig, SessionManagerStats, SessionMetadata, SessionState};
use super::SessionError;
use crate::connection::{ConnectionType, ConnectionConfig, ConnectionFactory};

/// Event types that can be emitted by sessions
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Session was created
    Created(SessionId),
    /// Session state changed
    StateChanged(SessionId, SessionState),
    /// Data was received
    DataReceived(SessionId, Vec<u8>),
    /// Data was sent
    DataSent(SessionId, usize),
    /// Error occurred
    Error(SessionId, String),
    /// Session was closed
    Closed(SessionId),
}

pub struct Session {
    id: SessionId,
    metadata: SessionMetadata,
    state: SessionState,
    event_sender: mpsc::Sender<SessionEvent>,
    created_at: std::time::Instant,
    last_activity: std::time::Instant,
    connection_index: Option<usize>,
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            metadata: self.metadata.clone(),
            state: self.state.clone(),
            event_sender: self.event_sender.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            connection_index: self.connection_index,
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("metadata", &self.metadata)
            .field("state", &self.state)
            .field("created_at", &self.created_at)
            .field("last_activity", &self.last_activity)
            .finish()
    }
}

impl Session {
    /// Get session ID
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Get session state
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Get session metadata
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    /// Get creation time
    pub fn created_at(&self) -> std::time::Instant {
        self.created_at
    }

    /// Get last activity time
    pub fn last_activity(&self) -> std::time::Instant {
        self.last_activity
    }

    /// Update last activity timestamp
    #[allow(dead_code)]
    fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    /// Get connection index
    pub fn connection_index(&self) -> Option<usize> {
        self.connection_index
    }

    /// Set connection index
    pub fn set_connection_index(&mut self, connection_index: Option<usize>) {
        self.connection_index = connection_index;
    }
}

/// Session manager for managing multiple debugging sessions
#[derive(Debug)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    config: SessionManagerConfig,
    stats: Arc<RwLock<SessionManagerStats>>,
    event_sender: mpsc::Sender<SessionEvent>,
    #[allow(dead_code)]
    event_receiver: Arc<RwLock<mpsc::Receiver<SessionEvent>>>,
    connection_registry: Arc<RwLock<super::ConnectionRegistry>>,
}

impl SessionManager {
    /// Create a new session manager with default configuration
    pub fn new() -> Self {
        let config = SessionManagerConfig::default();
        let (event_sender, event_receiver) = mpsc::channel(100);

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(SessionManagerStats::default())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(event_receiver)),
            connection_registry: Arc::new(RwLock::new(super::ConnectionRegistry::new())),
        }
    }

    /// Create a new session manager with custom configuration
    pub fn with_config(config: SessionManagerConfig) -> Self {
        let (event_sender, event_receiver) = mpsc::channel(100);

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(SessionManagerStats::default())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(event_receiver)),
            connection_registry: Arc::new(RwLock::new(super::ConnectionRegistry::new())),
        }
    }

    pub async fn create_session(
        &self,
        name: String,
        _connection_type: ConnectionType,
        _connection_config: ConnectionConfig,
    ) -> Result<SessionId, SessionError> {
        let sessions = self.sessions.read().await;
        if sessions.len() >= self.config.max_sessions {
            return Err(SessionError::MaxSessionsReached {
                max: self.config.max_sessions,
            });
        }
        drop(sessions);

        let session_id = format!("{}-{}", self.config.default_name_prefix, Uuid::new_v4());

        let metadata = SessionMetadata {
            id: session_id.clone(),
            name,
            connection_type: format!("{:?}", _connection_type),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        let (event_sender, _event_receiver) = mpsc::channel(100);

        let session = Session {
            id: session_id.clone(),
            metadata,
            state: SessionState::Created,
            event_sender,
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            connection_index: None,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        let mut stats = self.stats.write().await;
        stats.total_created += 1;
        stats.active_sessions = sessions.len();
        if sessions.len() > stats.peak_concurrent {
            stats.peak_concurrent = sessions.len();
        }

        let _ = self.event_sender.send(SessionEvent::Created(session_id.clone())).await;

        Ok(session_id)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn connect_session(
        &self,
        session_id: &SessionId,
        connection_config: ConnectionConfig,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| SessionError::NotFound {
            id: session_id.clone(),
        })?;

        if session.state() != &SessionState::Created {
            return Err(SessionError::InvalidState {
                id: session_id.clone(),
                state: format!("{:?}", session.state()),
            });
        }

        let connection: crate::connection::types::ConnectionHandle = match connection_config {
            ConnectionConfig::Serial(serial_config) => {
                std::sync::Arc::new(tokio::sync::Mutex::new(
                    ConnectionFactory::create_serial(serial_config)
                ))
            }
            ConnectionConfig::Telnet(telnet_config) => {
                std::sync::Arc::new(tokio::sync::Mutex::new(
                    ConnectionFactory::create_telnet(telnet_config)
                ))
            }
        };

        drop(sessions);

        {
            let mut conn = connection.lock().await;
            let conn_ref = &mut *conn;
            conn_ref.connect().await.map_err(|e| SessionError::CreationFailed {
                reason: format!("Failed to connect: {}", e),
            })?;
        }

        let connection_index = {
            let mut registry = self.connection_registry.write().await;
            registry.insert(
                session_id.clone(),
                connection,
            ).map_err(|e| SessionError::CreationFailed {
                reason: format!("Failed to insert connection: {}", e),
            })?
        };

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.set_connection_index(Some(connection_index));
                session.state = SessionState::Connected;
            }
        }

        let _ = self.event_sender.send(SessionEvent::StateChanged(session_id.clone(), SessionState::Connected)).await;

        Ok(())
    }

    pub async fn disconnect_session(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| SessionError::NotFound {
            id: session_id.clone(),
        })?;

        if session.state() != &SessionState::Connected {
            return Err(SessionError::InvalidState {
                id: session_id.clone(),
                state: format!("{:?}", session.state()),
            });
        }

        let connection_index = session.connection_index().ok_or_else(|| SessionError::NotConnected {
            id: session_id.clone(),
        })?;

        drop(sessions);

        {
            let mut registry = self.connection_registry.write().await;
            if let Some((_, connection)) = registry.remove_by_index(connection_index) {
                let mut conn = connection.lock().await;
                let _ = conn.disconnect().await;
            }
        }

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.set_connection_index(None);
                session.state = SessionState::Disconnected;
            }
        }

        let _ = self.event_sender.send(SessionEvent::StateChanged(session_id.clone(), SessionState::Disconnected)).await;

        Ok(())
    }

    pub async fn close_session(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions.get(session_id)
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.clone(),
            })?;

        let connection_index = session.connection_index();
        let state = session.state.clone();

        sessions.remove(session_id);

        let mut stats = self.stats.write().await;
        stats.total_destroyed += 1;
        stats.active_sessions = sessions.len();

        drop(sessions);

        if let Some(idx) = connection_index {
            // Always remove connection from registry to prevent resource leak
            let mut registry = self.connection_registry.write().await;
            if let Some((_, connection)) = registry.remove_by_index(idx) {
                // Only disconnect if the session was in Connected state
                if state == SessionState::Connected {
                    let mut conn = connection.lock().await;
                    let _ = conn.disconnect().await;
                }
            }
        }

        let _ = self.event_sender.send(SessionEvent::Closed(session_id.clone())).await;

        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    pub async fn stats(&self) -> SessionManagerStats {
        self.stats.read().await.clone()
    }

    pub fn config(&self) -> &SessionManagerConfig {
        &self.config
    }

    #[allow(dead_code)]
    pub async fn subscribe_events(&self) -> mpsc::Receiver<SessionEvent> {
        let (_tx, rx) = mpsc::channel(100);
        rx
    }

    pub async fn write_session_data(
        &self,
        session_id: &SessionId,
        data: Vec<u8>,
    ) -> Result<usize, SessionError> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id)
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.clone(),
            })?;

        if session.state() != &SessionState::Connected {
            return Err(SessionError::NotConnected {
                id: session_id.clone(),
            });
        }

        let connection_index = session.connection_index().ok_or_else(|| SessionError::NotConnected {
            id: session_id.clone(),
        })?;

        drop(sessions);

        let connection = {
            let registry = self.connection_registry.read().await;
            registry.get_by_index(connection_index).ok_or_else(|| SessionError::NotConnected {
                id: session_id.clone(),
            })?
        };

        let bytes_written = {
            let mut conn = connection.lock().await;
            let conn_ref = &mut *conn;
            conn_ref.write(&data).await.map_err(|e| SessionError::Generic {
                0: format!("Write failed: {}", e),
            })?
        };

        let _ = self.event_sender.send(SessionEvent::DataSent(session_id.clone(), bytes_written)).await;

        Ok(bytes_written)
    }

    pub async fn rename_session(
        &self,
        session_id: &SessionId,
        new_name: String,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.clone(),
            })?;
        
        session.metadata.set_name(new_name);
        
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::types::{SerialConfig, DataBits, Parity, StopBits, FlowControl};

    #[test]
    fn test_session_manager_creation() {
        let manager = SessionManager::new();
        assert_eq!(manager.config().max_sessions, 10);
    }

    #[test]
    fn test_session_manager_with_custom_config() {
        let config = SessionManagerConfig {
            max_sessions: 5,
            default_name_prefix: "Test".to_string(),
            auto_cleanup: false,
            session_timeout_secs: 1800,
        };
        
        let manager = SessionManager::with_config(config);
        assert_eq!(manager.config().max_sessions, 5);
        assert_eq!(manager.config().default_name_prefix, "Test");
        assert!(!manager.config().auto_cleanup);
    }

    #[tokio::test]
    async fn test_session_creation_and_listing() {
        let manager = SessionManager::new();
        
        // Initially no sessions
        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());
        
        // Create a session
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        });
        
        let session_id = manager.create_session(
            "Test Session".to_string(),
            ConnectionType::Serial,
            connection_config,
        ).await.expect("Failed to create session");
        
        // Verify session was created
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session_id);
        
        // Get specific session
        let session = manager.get_session(&session_id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().metadata.name, "Test Session");
    }

    #[tokio::test]
    async fn test_session_closure() {
        let manager = SessionManager::new();
        
        // Create a session
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        });
        
        let session_id = manager.create_session(
            "Test Session".to_string(),
            ConnectionType::Serial,
            connection_config,
        ).await.expect("Failed to create session");
        
        // Verify session exists
        assert!(manager.get_session(&session_id).await.is_some());
        
        // Close the session
        manager.close_session(&session_id).await.expect("Failed to close session");
        
        // Verify session was removed
        assert!(manager.get_session(&session_id).await.is_none());
        
        // Verify stats were updated
        let stats = manager.stats().await;
        assert_eq!(stats.total_destroyed, 1);
        assert_eq!(stats.active_sessions, 0);
    }

    #[tokio::test]
    async fn test_max_sessions_limit() {
        // Create manager with max 2 sessions
        let config = SessionManagerConfig {
            max_sessions: 2,
            default_name_prefix: "Test".to_string(),
            auto_cleanup: true,
            session_timeout_secs: 3600,
        };
        let manager = SessionManager::with_config(config);
        
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        });
        
        // Create first two sessions (should succeed)
        let session1 = manager.create_session(
            "Session 1".to_string(),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await;
        assert!(session1.is_ok());
        
        let session2 = manager.create_session(
            "Session 2".to_string(),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await;
        assert!(session2.is_ok());
        
        // Third session should fail due to limit
        let session3 = manager.create_session(
            "Session 3".to_string(),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await;
        assert!(session3.is_err());
        assert!(matches!(session3.unwrap_err(), SessionError::MaxSessionsReached { .. }));
    }

    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let manager = SessionManager::new();
        
        // Try to close a session that doesn't exist
        let result = manager.close_session(&"nonexistent-id".to_string()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::NotFound { .. }));
    }
}