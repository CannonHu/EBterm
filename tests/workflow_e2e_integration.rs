//! End-to-end workflow integration tests
//!
//! Tests complete debug workflows from session creation to cleanup

use embedded_debugger::{
    command::{CommandManager, DefaultCommandParser, DefaultCommandManager},
    connection::{ConnectionConfig, SerialConfig, ConnectionType},
    logger::{FileLogger, Logger},
    session::{SessionManager},
};

mod common;
use common::*;

// CM-003: Full debug workflow - complete end-to-end workflow
#[tokio::test]
async fn test_full_debug_workflow() {
    // Given: SessionManager is created
    let session_manager = SessionManager::new();

    // And: CommandManager is created with parser
    let parser = Box::new(DefaultCommandParser::default());
    let command_manager = DefaultCommandManager::new(parser);

    // And: FileLogger is created
    let mut logger = FileLogger::new();

    // And: Temporary directory is created for test files
    let temp_dir = create_test_dir();

    // When: A session is created (no actual connection needed for workflow testing)
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
            "debug-session".to_string(),
            ConnectionType::Serial,
            connection_config,
        )
        .await
        .expect("Failed to create session");

    // Then: Session exists and is in Created state
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session should exist");
    assert_eq!(
        session.state(),
        &embedded_debugger::session::SessionState::Created
    );
    assert_eq!(session.metadata().name, "debug-session");

    // When: A temporary log file is created and logging is started
    let log_file = temp_dir.path().join("debug.log");
    logger
        .start_logging(&log_file)
        .await
        .expect("Failed to start logging");
    assert!(logger.is_logging());

    // Then: Log file path is set
    assert_eq!(logger.current_log_path(), Some(log_file.as_path()));

    // When: Commands are loaded from a temporary file
    let command_content = r#"show version
show running-config
show interfaces
show ip route
show arp
show vlan brief
"#;
    let command_file = create_test_file(&temp_dir, "commands.txt", command_content);

    let command_count = command_manager
        .load_from_file(&command_file)
        .await
        .expect("Failed to load commands");

    // Then: Commands are loaded successfully
    assert_eq!(command_count, 6);
    assert_eq!(command_manager.count().await, 6);

    // When: Session is "connected" (simulated)
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session should still exist");
    assert_eq!(
        session.state(),
        &embedded_debugger::session::SessionState::Created
    );

    // When: Three commands are executed through the session
    let mut execution_results = Vec::new();
    for i in 0..3 {
        // Log command as input
        let command = command_manager
            .get_command(i)
            .await
            .expect("Command should exist");
        let command_bytes = format!("{}\r\n", command.content).into_bytes();
        logger
            .log_input(&session_id, &command_bytes)
            .await
            .expect("Failed to log input");

        // Execute command
        let result = command_manager
            .execute(i)
            .await
            .expect("Failed to execute command");
        execution_results.push(result.clone());

        // Log output
        let output_bytes = format!("{}\r\nOK\r\n", result.output).into_bytes();
        logger
            .log_output(&session_id, &output_bytes)
            .await
            .expect("Failed to log output");
    }

    // Then: All three commands executed successfully
    assert_eq!(execution_results.len(), 3);
    assert!(execution_results.iter().all(|r| r.success));

    // And: Command outputs contain expected content
    assert!(execution_results[0].output.contains("show version"));
    assert!(execution_results[1].output.contains("show running-config"));
    assert!(execution_results[2].output.contains("show interfaces"));

    // When: Log file content is read
    let log_contents = std::fs::read_to_string(&log_file)
        .expect("Failed to read log file");

    // Then: Log file contains direction markers
    assert!(log_contents.contains("[INPUT]"));
    assert!(log_contents.contains("[OUTPUT]"));

    // And: Log file has data (commands logged as hex)
    assert!(log_contents.len() > 100);

    // And: Logger stats reflect the logged data
    let stats = logger.stats();
    assert_eq!(stats.total_entries, 6); // 3 input + 3 output
    assert!(stats.bytes_logged_input > 0);
    assert!(stats.bytes_logged_output > 0);

    // When: Logger is stopped
    logger
        .stop_logging()
        .await
        .expect("Failed to stop logging");
    assert!(!logger.is_logging());

    // When: Session is "disconnected" (simulated)
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session should still exist");
    assert_eq!(
        session.state(),
        &embedded_debugger::session::SessionState::Created
    );

    // When: Session is closed
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");

    // Then: Session is removed (not found)
    assert!(
        session_manager.get_session(&session_id).await.is_none(),
        "Session should be removed after close"
    );

    // And: Session manager stats reflect the cleanup
    let session_stats = session_manager.stats().await;
    assert_eq!(session_stats.total_created, 1);
    assert_eq!(session_stats.total_destroyed, 1);
    assert_eq!(session_stats.active_sessions, 0);

    // And: Log file still exists with all data
    assert!(log_file.exists());
    let final_log_contents = std::fs::read_to_string(&log_file)
        .expect("Failed to read final log file");
    assert_eq!(final_log_contents, log_contents);

    // Then: All resources are released (verified by stats)
    assert_eq!(command_manager.count().await, 6); // Commands still in memory
    assert!(!logger.is_logging()); // Logger stopped
    assert_eq!(session_stats.active_sessions, 0); // Session closed
}
