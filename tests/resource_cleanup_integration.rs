//! Resource cleanup integration tests
//!
//! Tests proper cleanup of resources (connections, loggers, memory) on session closure

use embedded_debugger::connection::{ConnectionConfig, SerialConfig, TelnetConfig, ConnectionType};
use embedded_debugger::logger::{FileLogger, Logger};
use embedded_debugger::session::SessionManager;

mod common;
use common::*;

// RC-001: Verify session.close() releases connections
// Tests that close_session properly cleans up session resources
#[tokio::test]
async fn test_session_close_releases_connections() {
    // Given: A SessionManager is created
    let session_manager = SessionManager::new();

    // And: Initial stats show no sessions
    let initial_stats = session_manager.stats().await;
    assert_eq!(initial_stats.total_created, 0);
    assert_eq!(initial_stats.total_destroyed, 0);
    assert_eq!(initial_stats.active_sessions, 0);

    // When: A session with SerialConfig is created
    let serial_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/null".to_string(),
        baud_rate: 115200,
        data_bits: embedded_debugger::connection::DataBits::Eight,
        parity: embedded_debugger::connection::Parity::None,
        stop_bits: embedded_debugger::connection::StopBits::One,
        flow_control: embedded_debugger::connection::FlowControl::None,
    });

    let serial_session_id = session_manager
        .create_session(
            "serial-session".to_string(),
            ConnectionType::Serial,
            serial_config,
        )
        .await
        .expect("Failed to create serial session");

    // And: A session with TelnetConfig is created
    let telnet_config = ConnectionConfig::Telnet(TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: 23,
        connect_timeout_secs: 60,
    });

    let telnet_session_id = session_manager
        .create_session(
            "telnet-session".to_string(),
            ConnectionType::Telnet,
            telnet_config,
        )
        .await
        .expect("Failed to create telnet session");

    // Then: Both sessions exist
    let stats_after_create = session_manager.stats().await;
    assert_eq!(stats_after_create.total_created, 2);
    assert_eq!(stats_after_create.active_sessions, 2);

    // When: Both sessions are closed
    session_manager
        .close_session(&serial_session_id)
        .await
        .expect("Failed to close serial session");
    session_manager
        .close_session(&telnet_session_id)
        .await
        .expect("Failed to close telnet session");

    // Then: Sessions are removed
    assert!(session_manager.get_session(&serial_session_id).await.is_none());
    assert!(session_manager.get_session(&telnet_session_id).await.is_none());

    // And: Stats show both sessions destroyed
    let stats_after_close = session_manager.stats().await;
    assert_eq!(stats_after_close.total_created, 2);
    assert_eq!(stats_after_close.total_destroyed, 2);
    assert_eq!(stats_after_close.active_sessions, 0);

    // And: No resource leaks (stats balanced)
    assert_eq!(
        stats_after_close.total_created,
        stats_after_close.total_destroyed
    );
}

// RC-002: Verify logger cleanup on session close
// Tests that when a session is closed, logging can be properly stopped
#[tokio::test]
async fn test_logger_cleanup_on_session_close() {
    // Given: A SessionManager and FileLogger are created
    let session_manager = SessionManager::new();
    let mut logger = FileLogger::new();

    // And: A temporary log file is created
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("cleanup_test.log");

    // And: A session is created
    let connection_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/null".to_string(),
        baud_rate: 115200,
        data_bits: embedded_debugger::connection::DataBits::Eight,
        parity: embedded_debugger::connection::Parity::None,
        stop_bits: embedded_debugger::connection::StopBits::One,
        flow_control: embedded_debugger::connection::FlowControl::None,
    });

    let session_id = session_manager
        .create_session(
            "test-session".to_string(),
            ConnectionType::Serial,
            connection_config,
        )
        .await
        .expect("Failed to create session");

    // When: Logger starts logging
    logger
        .start_logging(&log_file)
        .await
        .expect("Failed to start logging");
    assert!(logger.is_logging());

    // And: Some data is written to the log
    logger
        .log_input(&session_id, b"Test data\r\n")
        .await
        .expect("Failed to log input");
    logger
        .log_output(&session_id, b"Test response\r\n")
        .await
        .expect("Failed to log output");

    // And: Session is closed (simulating cleanup on session end)
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    // And: Logger is stopped (simulating cleanup)
    logger
        .stop_logging()
        .await
        .expect("Failed to stop logging");

    // Then: Logger is no longer logging
    assert!(!logger.is_logging());

    // And: Session is removed
    assert!(session_manager.get_session(&session_id).await.is_none());

    // And: Log file contains the logged data
    assert!(log_file.exists());
    let contents = std::fs::read_to_string(&log_file).expect("Failed to read log file");
    assert!(contents.contains("[INPUT]"));
    assert!(contents.contains("[OUTPUT]"));
}

// RC-003: Verify rapid create/close cycles don't leak memory
// Tests that repeated session creation and closure properly cleans up resources
#[tokio::test]
async fn test_rapid_create_close_cycles_no_memory_leak() {
    // Given: A SessionManager is created
    let session_manager = SessionManager::new();

    // And: Initial stats are zero
    let initial_stats = session_manager.stats().await;
    assert_eq!(initial_stats.total_created, 0);
    assert_eq!(initial_stats.total_destroyed, 0);
    assert_eq!(initial_stats.active_sessions, 0);

    // When: 50 sessions are created and immediately closed
    let cycle_count = 50;
    for i in 0..cycle_count {
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: format!("/dev/ttyUSB{}", i % 10),
            baud_rate: 115200,
            data_bits: embedded_debugger::connection::DataBits::Eight,
            parity: embedded_debugger::connection::Parity::None,
            stop_bits: embedded_debugger::connection::StopBits::One,
            flow_control: embedded_debugger::connection::FlowControl::None,
        });

        let session_id = session_manager
            .create_session(
                format!("session-{}", i),
                ConnectionType::Serial,
                connection_config,
            )
            .await
            .expect(&format!("Failed to create session {}", i));

        // Immediately close the session
        session_manager
            .close_session(&session_id)
            .await
            .expect(&format!("Failed to close session {}", i));
    }

    // Then: All sessions are properly cleaned up
    let final_stats = session_manager.stats().await;
    assert_eq!(
        final_stats.total_created, cycle_count as u64,
        "Expected {} total created sessions",
        cycle_count
    );
    assert_eq!(
        final_stats.total_destroyed, cycle_count as u64,
        "Expected {} total destroyed sessions",
        cycle_count
    );
    assert_eq!(
        final_stats.active_sessions, 0,
        "Expected 0 active sessions after cleanup"
    );

    // And: No memory leaks (all sessions accounted for)
    assert_eq!(
        final_stats.total_created, final_stats.total_destroyed,
        "Created and destroyed counts should match (no leaks)"
    );

    // And: No active sessions remain
    let sessions = session_manager.list_sessions().await;
    assert!(
        sessions.is_empty(),
        "Expected no active sessions, found {}",
        sessions.len()
    );
}

// Additional test: Verify connection registry is cleaned up after session close
#[tokio::test]
async fn test_connection_registry_cleanup_on_close() {
    // Given: A SessionManager is created
    let session_manager = SessionManager::new();

    // When: Multiple sessions are created and closed
    for i in 0..10 {
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: format!("/dev/ttyUSB{}", i),
            baud_rate: 115200,
            data_bits: embedded_debugger::connection::DataBits::Eight,
            parity: embedded_debugger::connection::Parity::None,
            stop_bits: embedded_debugger::connection::StopBits::One,
            flow_control: embedded_debugger::connection::FlowControl::None,
        });

        let session_id = session_manager
            .create_session(
                format!("session-{}", i),
                ConnectionType::Serial,
                connection_config,
            )
            .await
            .expect(&format!("Failed to create session {}", i));

        session_manager
            .close_session(&session_id)
            .await
            .expect(&format!("Failed to close session {}", i));
    }

    // Then: Connection registry should be empty (all connections removed)
    let registry = session_manager.connection_registry();
    let registry_read = registry.read().await;
    assert_eq!(
        registry_read.len(),
        0,
        "Expected empty connection registry after all sessions closed"
    );

    // And: Session stats show proper cleanup
    let stats = session_manager.stats().await;
    assert_eq!(stats.active_sessions, 0);
    assert_eq!(stats.total_created, 10);
    assert_eq!(stats.total_destroyed, 10);
}