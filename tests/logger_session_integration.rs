//! Logger and session integration tests
//!
//! Tests integration between Logger and SessionManager for logging during active sessions

use embedded_debugger::logger::{FileLogger, Logger};
use embedded_debugger::connection::{ConnectionConfig, SerialConfig, ConnectionType};
use embedded_debugger::session::SessionManager;

mod common;
use common::*;

// CM-002: Logging during active session
#[tokio::test]
async fn test_logging_during_active_session() {
    // Given: SessionManager and FileLogger are initialized
    let session_manager = SessionManager::new();
    let mut logger = FileLogger::new();

    // And: A temporary log file path is created
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("session.log");

    // And: A session is created (no actual connection needed for logging)
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

    // Verify session exists in Created state
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);

    // When: Logger starts logging to the temporary file
    logger.start_logging(&log_file)
        .await
        .expect("Failed to start logging");
    assert!(logger.is_logging());
    assert_eq!(logger.current_log_path(), Some(log_file.as_path()));

    // And: Test data is logged as input
    let input_data = b"AT+GMR\r\n";
    logger.log_input(&session_id, input_data)
        .await
        .expect("Failed to log input");

    // And: Test data is logged as output
    let output_data = b"ESP8266 Module Version 1.0.0\r\nOK\r\n";
    logger.log_output(&session_id, output_data)
        .await
        .expect("Failed to log output");

    // And: More data is logged in both directions
    logger.log_input(&session_id, b"AT+CWLAP\r\n")
        .await
        .expect("Failed to log input");
    let cwlap_output = b"+CWLAP: (4,\"MyWiFi\",-70,\"11:22:33:44:55:66\",1)\r\nOK\r\n";
    logger.log_output(&session_id, cwlap_output)
        .await
        .expect("Failed to log output");

    // Then: Logger stats reflect the logged data
    let stats = logger.stats();
    assert_eq!(stats.bytes_logged_input, (input_data.len() + b"AT+CWLAP\r\n".len()) as u64);
    assert_eq!(stats.bytes_logged_output, (output_data.len() + cwlap_output.len()) as u64);
    assert_eq!(stats.total_entries, 4);

    // And: Log file exists and contains the logged data
    assert!(log_file.exists());
    let contents = std::fs::read_to_string(&log_file).expect("Failed to read log file");

    // Verify log format contains direction markers
    assert!(contents.contains("[INPUT]"));
    assert!(contents.contains("[OUTPUT]"));

    // Verify session is still active
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);

    // When: Logger is stopped
    logger.stop_logging()
        .await
        .expect("Failed to stop logging");
    assert!(!logger.is_logging());

    // And: Session is closed
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    // Then: Session is removed
    assert!(session_manager.get_session(&session_id).await.is_none());
}

// CM-005: Logger failure during session - graceful degradation
#[tokio::test]
async fn test_logger_failure_during_session() {
    // Given: SessionManager and FileLogger are initialized
    let session_manager = SessionManager::new();
    let mut logger = FileLogger::new();

    // And: A temporary log file path is created
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("session.log");

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

    // Verify session exists
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);

    // When: Logger starts logging
    logger.start_logging(&log_file)
        .await
        .expect("Failed to start logging");

    // And: Initial data is logged successfully
    logger.log_input(&session_id, b"Initial data\r\n")
        .await
        .expect("Failed to log initial input");
    logger.log_output(&session_id, b"Initial response\r\n")
        .await
        .expect("Failed to log initial output");

    let stats_before_failure = logger.stats();
    assert_eq!(stats_before_failure.total_entries, 2);

    // When: Logger is stopped (simulating failure by disabling logging)
    logger.stop_logging()
        .await
        .expect("Failed to stop logging");
    assert!(!logger.is_logging());

    // And: Attempt to log more data (should fail gracefully)
    let log_input_result = logger.log_input(&session_id, b"Data after failure\r\n").await;

    // Then: Logging fails with NotEnabled error (graceful degradation)
    assert!(log_input_result.is_err(), "Logging should fail when not enabled");
    let log_input_err = log_input_result.unwrap_err();
    match log_input_err {
        embedded_debugger::logger::LoggerError::NotEnabled { session_id: sid } => {
            assert_eq!(sid, session_id);
        }
        _ => panic!("Expected NotEnabled error, got {:?}", log_input_err),
    }

    // And: Stats remain unchanged (failed log didn't update stats)
    let stats_after_failure = logger.stats();
    assert_eq!(stats_after_failure.total_entries, stats_before_failure.total_entries);
    assert_eq!(stats_after_failure.bytes_logged_input, stats_before_failure.bytes_logged_input);
    assert_eq!(stats_after_failure.bytes_logged_output, stats_before_failure.bytes_logged_output);

    // Critical: Session continues to operate despite logger failure
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);
    assert_eq!(session.metadata().name, "test-session");

    // And: Session operations still work (e.g., rename)
    session_manager
        .rename_session(&session_id, "graceful-session".to_string())
        .await
        .expect("Failed to rename session");

    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.metadata().name, "graceful-session");

    // And: Session can be listed
    let sessions = session_manager.list_sessions().await;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].metadata().name, "graceful-session");

    // And: Session can be closed normally
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    assert!(session_manager.get_session(&session_id).await.is_none());
}

// Additional test: Logger with multiple concurrent sessions
#[tokio::test]
async fn test_logger_with_multiple_concurrent_sessions() {
    // Given: SessionManager and FileLogger are initialized
    let session_manager = SessionManager::new();
    let mut logger = FileLogger::new();

    // And: A temporary log file is created
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("multi_session.log");

    // And: Multiple sessions are created
    let mut session_ids = Vec::new();
    for i in 0..3 {
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
            .expect("Failed to create session");
        session_ids.push(session_id);
    }

    // When: Logger starts logging
    logger.start_logging(&log_file)
        .await
        .expect("Failed to start logging");

    // And: Data is logged for each session
    for (i, session_id) in session_ids.iter().enumerate() {
        let input_data = format!("Session {} input\r\n", i).into_bytes();
        let output_data = format!("Session {} response\r\n", i).into_bytes();

        logger.log_input(session_id, &input_data)
            .await
            .expect("Failed to log input");
        logger.log_output(session_id, &output_data)
            .await
            .expect("Failed to log output");
    }

    // Then: Logger stats track all entries across sessions
    let stats = logger.stats();
    assert_eq!(stats.total_entries, 6); // 3 sessions * 2 entries each

    // And: all sessions remain active
    let sessions = session_manager.list_sessions().await;
    assert_eq!(sessions.len(), 3);

    // Cleanup
    for session_id in &session_ids {
        session_manager
            .close_session(session_id)
            .await
            .expect("Failed to close session");
    }
    logger.stop_logging().await.ok(); // Ignore errors during cleanup
}

// Additional test: Logger rotation during active session
#[tokio::test]
async fn test_logger_rotation_during_active_session() {
    // Given: SessionManager and FileLogger are initialized with small max file size
    let session_manager = SessionManager::new();
    let config = embedded_debugger::logger::LoggerConfig {
        max_file_size: 100, // Very small to trigger rotation
        max_backup_files: 2,
        compress_rotated: false,
        buffer_size: 4096,
    };
    let mut logger = FileLogger::with_config(config);

    // And: A temporary log file is created
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("rotation.log");

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
    logger.start_logging(&log_file)
        .await
        .expect("Failed to start logging");

    // And: Large data is logged to trigger rotation
    let large_data = vec![0x41u8; 200]; // 200 bytes of 'A'
    logger.log_input(&session_id, &large_data)
        .await
        .expect("Failed to log large input");

    // Then: Original log file should exist (rotation creates new file)
    assert!(log_file.exists());

    // And: Session remains active after rotation
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);

    // Cleanup
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");
    logger.stop_logging().await.ok();
}
