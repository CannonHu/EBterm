//! Session manager integration tests
//!
//! Tests full session lifecycle, concurrency, callbacks, and state management

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashSet;

use embedded_debugger::session::types::{SessionState, SessionManagerConfig};
use embedded_debugger::session::SessionManager;
use embedded_debugger::session::SessionCallbacks;
use embedded_debugger::connection::{ConnectionConfig, SerialConfig, TelnetConfig, ConnectionType};

// SI-001: Full lifecycle test (create→close only, since connect requires actual hardware)
#[tokio::test]
async fn test_session_full_lifecycle() {
    // Create session manager
    let manager = SessionManager::new();

    // Verify initial stats
    let stats = manager.stats().await;
    assert_eq!(stats.total_created, 0);
    assert_eq!(stats.active_sessions, 0);

    // Create a session
    let connection_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/null".to_string(),
        baud_rate: 115200,
        data_bits: embedded_debugger::connection::DataBits::Eight,
        parity: embedded_debugger::connection::Parity::None,
        stop_bits: embedded_debugger::connection::StopBits::One,
        flow_control: embedded_debugger::connection::FlowControl::None,
    });

    let session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    let stats = manager.stats().await;
    assert_eq!(stats.total_created, 1);
    assert_eq!(stats.active_sessions, 1);

    let session = manager.get_session(&session_id).await.expect("Session not found");
    assert_eq!(session.state(), &SessionState::Created);
    assert_eq!(session.metadata().name, "test-session");

    manager.close_session(&session_id).await.expect("Failed to close session");

    assert!(manager.get_session(&session_id).await.is_none());
    let stats = manager.stats().await;
    assert_eq!(stats.total_destroyed, 1);
    assert_eq!(stats.active_sessions, 0);
}

// SI-002: Concurrent sessions isolation
#[tokio::test]
async fn test_concurrent_sessions_isolation() {
    let manager = SessionManager::new();
    let mut session_ids = Vec::new();

    for i in 0..5 {
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: format!("/dev/ttyUSB{}", i),
            baud_rate: 115200,
            data_bits: embedded_debugger::connection::DataBits::Eight,
            parity: embedded_debugger::connection::Parity::None,
            stop_bits: embedded_debugger::connection::StopBits::One,
            flow_control: embedded_debugger::connection::FlowControl::None,
        });

        let id = manager.create_session(
            format!("session-{}", i),
            ConnectionType::Serial,
            connection_config,
        ).await.expect("Failed to create session");
        session_ids.push(id);
    }

    for (i, id) in session_ids.iter().enumerate() {
        let session = manager.get_session(id).await.expect("Session not found");
        assert_eq!(session.state(), &SessionState::Created);
        assert_eq!(session.metadata().name, format!("session-{}", i));
    }

    let stats = manager.stats().await;
    assert_eq!(stats.peak_concurrent, 5);

    for id in &session_ids {
        manager.close_session(id).await.expect("Failed to close session");
    }

    // Verify all are removed
    for id in &session_ids {
        assert!(manager.get_session(id).await.is_none());
    }
    let stats = manager.stats().await;
    assert_eq!(stats.total_created, 5);
    assert_eq!(stats.total_destroyed, 5);
    assert_eq!(stats.active_sessions, 0);
}

// SI-003: Callbacks invocation
#[tokio::test]
async fn test_session_callbacks_invocation() {
    let on_connected_called = Arc::new(AtomicBool::new(false));
    let on_disconnected_called = Arc::new(AtomicBool::new(false));
    let on_error_called = Arc::new(AtomicBool::new(false));

    let callbacks = SessionCallbacks::new()
        .with_on_connected({
            let on_connected_called = on_connected_called.clone();
            move |session_id, _registry| {
                assert!(!session_id.is_empty());
                on_connected_called.store(true, Ordering::SeqCst);
            }
        })
        .with_on_disconnected({
            let on_disconnected_called = on_disconnected_called.clone();
            move |session_id| {
                assert!(!session_id.is_empty());
                on_disconnected_called.store(true, Ordering::SeqCst);
            }
        })
        .with_on_error({
            let on_error_called = on_error_called.clone();
            move |session_id, error| {
                assert!(!session_id.is_empty());
                assert!(!error.is_empty());
                on_error_called.store(true, Ordering::SeqCst);
            }
        });

    let manager = SessionManager::with_callbacks(callbacks);

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    let session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    assert!(!on_connected_called.load(Ordering::SeqCst));
    assert!(!on_disconnected_called.load(Ordering::SeqCst));
    assert!(!on_error_called.load(Ordering::SeqCst));
    let _ = manager.close_session(&session_id).await;
}

// SI-004: State transitions
#[tokio::test]
async fn test_session_state_transitions() {
    let manager = SessionManager::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    let session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    let session = manager.get_session(&session_id).await.expect("Session not found");
    assert_eq!(session.state(), &SessionState::Created);

    manager.close_session(&session_id).await.expect("Failed to close session");
    assert!(manager.get_session(&session_id).await.is_none());
}

// SI-005: Session ID uniqueness
#[tokio::test]
async fn test_session_id_uniqueness() {
    let manager = SessionManager::new();
    let mut session_ids = HashSet::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    for i in 0..10 {
        let id = manager.create_session(
            format!("test-session-{}", i),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await.expect("Failed to create session");

        assert!(!session_ids.contains(&id), "Duplicate session ID found: {}", id);
        session_ids.insert(id);
    }

    assert_eq!(session_ids.len(), 10);
    for id in session_ids {
        let _ = manager.close_session(&id).await;
    }
}

// SI-006: Write to disconnected session
#[tokio::test]
async fn test_write_to_disconnected_session() {
    let manager = SessionManager::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    let session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    let result = manager.write_session_data(&session_id, b"test data".to_vec()).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        embedded_debugger::session::SessionError::NotConnected { id } => {
            assert_eq!(id, session_id);
        }
        _ => panic!("Expected NotConnected error"),
    }
    let _ = manager.close_session(&session_id).await;
}

// SI-007: Session reconnect behavior
#[tokio::test]
async fn test_session_reconnect() {
    let manager = SessionManager::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    let session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    let session = manager.get_session(&session_id).await.expect("Session not found");
    assert_eq!(session.state(), &SessionState::Created);

    manager.close_session(&session_id).await.expect("Failed to close session");

    assert!(manager.get_session(&session_id).await.is_none());

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());
    let result = manager.connect_session(&session_id, connection_config).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        embedded_debugger::session::SessionError::NotFound { id } => {
            assert_eq!(id, session_id);
        }
        _ => panic!("Expected NotFound error"),
    }

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());
    let new_session_id = manager.create_session(
        "test-session".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create new session");

    let new_session = manager.get_session(&new_session_id).await.expect("New session not found");
    assert_eq!(new_session.state(), &SessionState::Created);

    assert_ne!(session_id, new_session_id);
    let _ = manager.close_session(&new_session_id).await;
}

// Additional test: Session manager with custom config
#[tokio::test]
async fn test_session_manager_custom_config() {
    let config = SessionManagerConfig {
        max_sessions: 3,
        default_name_prefix: "TestSession".to_string(),
        auto_cleanup: false,
        session_timeout_secs: 1800,
    };

    let manager = SessionManager::with_config(config);

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    for i in 0..3 {
        let id = manager.create_session(
            format!("session-{}", i),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await;
        assert!(id.is_ok(), "Failed to create session {}", i);
    }
    let result = manager.create_session(
        "session-3".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        embedded_debugger::session::SessionError::MaxSessionsReached { max } => {
            assert_eq!(max, 3);
        }
        _ => panic!("Expected MaxSessionsReached error"),
    }
}

// Additional test: Session rename
#[tokio::test]
async fn test_session_rename() {
    let manager = SessionManager::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    let session_id = manager.create_session(
        "original-name".to_string(),
        ConnectionType::Serial,
        connection_config,
    ).await.expect("Failed to create session");

    let session = manager.get_session(&session_id).await.expect("Session not found");
    assert_eq!(session.metadata().name, "original-name");

    manager.rename_session(&session_id, "new-name".to_string())
        .await.expect("Failed to rename session");

    let session = manager.get_session(&session_id).await.expect("Session not found");
    assert_eq!(session.metadata().name, "new-name");
    let _ = manager.close_session(&session_id).await;
}

// Additional test: List sessions
#[tokio::test]
async fn test_list_sessions() {
    let manager = SessionManager::new();

    let sessions = manager.list_sessions().await;
    assert!(sessions.is_empty());

    let connection_config = ConnectionConfig::Serial(SerialConfig::default());

    for i in 0..3 {
        let _ = manager.create_session(
            format!("session-{}", i),
            ConnectionType::Serial,
            connection_config.clone(),
        ).await.expect("Failed to create session");
    }

    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 3);
    for session in &sessions {
        assert_eq!(session.state(), &SessionState::Created);
    }
}

// Additional test: ConnectionConfig variants
#[tokio::test]
async fn test_connection_config_variants() {
    let manager = SessionManager::new();

    let serial_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 9600,
        data_bits: embedded_debugger::connection::DataBits::Seven,
        parity: embedded_debugger::connection::Parity::Even,
        stop_bits: embedded_debugger::connection::StopBits::Two,
        flow_control: embedded_debugger::connection::FlowControl::Hardware,
    });

    let serial_session = manager.create_session(
        "serial-session".to_string(),
        ConnectionType::Serial,
        serial_config,
    ).await;
    assert!(serial_session.is_ok());

    let telnet_config = ConnectionConfig::Telnet(TelnetConfig {
        host: "example.com".to_string(),
        port: 2323,
        connect_timeout_secs: 60,
    });

    let telnet_session = manager.create_session(
        "telnet-session".to_string(),
        ConnectionType::Telnet,
        telnet_config,
    ).await;
    assert!(telnet_session.is_ok());
    if let Ok(id) = serial_session {
        let _ = manager.close_session(&id).await;
    }
    if let Ok(id) = telnet_session {
        let _ = manager.close_session(&id).await;
    }
}
