//! Command and session integration tests
//!
//! Tests integration between CommandManager and SessionManager for command execution workflows

use embedded_debugger::command::{CommandManager, DefaultCommandParser, DefaultCommandManager};
use embedded_debugger::connection::{ConnectionConfig, SerialConfig, ConnectionType};
use embedded_debugger::session::SessionManager;

mod common;
use common::*;

// CM-001: Command execution through session
#[tokio::test]
async fn test_command_execution_through_session() {
    // Given: SessionManager and CommandManager are initialized
    let session_manager = SessionManager::new();
    let parser = Box::new(DefaultCommandParser::default());
    let command_manager = DefaultCommandManager::new(parser);

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

    // And: A temporary command file with test commands is created
    let temp_dir = create_test_dir();
    let command_content = r#"
show version
show running-config
show interfaces
ping 192.168.1.1
traceroute 8.8.8.8
"#;
    let command_file = create_test_file(&temp_dir, "commands.txt", command_content);

    // When: Commands are loaded from file
    let count = command_manager
        .load_from_file(&command_file)
        .await
        .expect("Failed to load commands");

    // Then: Correct number of commands loaded
    assert_eq!(count, 5);
    assert_eq!(command_manager.count().await, 5);

    // When: Commands are executed sequentially
    let mut execution_results = Vec::new();
    for i in 0..count {
        let result = command_manager
            .execute(i)
            .await
            .expect("Failed to execute command");
        execution_results.push(result);
    }

    // Then: All commands executed successfully
    assert_eq!(execution_results.len(), 5);

    // And: Each execution result indicates success
    assert!(execution_results.iter().all(|r| r.success));

    // And: Output contains expected command prefixes
    assert!(execution_results[0].output.contains("show version"));
    assert!(execution_results[1].output.contains("show running-config"));
    assert!(execution_results[2].output.contains("show interfaces"));
    assert!(execution_results[3].output.contains("ping 192.168.1.1"));
    assert!(execution_results[4].output.contains("traceroute 8.8.8.8"));

    // And: Session still exists in Created state
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
}

// CM-004: Command with broken connection error handling
#[tokio::test]
async fn test_command_with_broken_connection() {
    // Given: SessionManager and CommandManager are initialized
    let session_manager = SessionManager::new();
    let parser = Box::new(DefaultCommandParser::default());
    let command_manager = DefaultCommandManager::new(parser);

    // And: A session is created (disconnected state)
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

    // And: Commands are loaded
    command_manager
        .load_from_string("show version\nshow stats")
        .await
        .expect("Failed to load commands");

    // When: Attempting to write to disconnected session
    let write_result = session_manager
        .write_session_data(&session_id, b"test data".to_vec())
        .await;

    // Then: Write fails with NotConnected error
    assert!(write_result.is_err());
    match write_result.unwrap_err() {
        embedded_debugger::session::SessionError::NotConnected { id } => {
            assert_eq!(id, session_id);
        }
        _ => panic!("Expected NotConnected error"),
    }

    // And: Commands can still be executed by CommandManager (simulated execution)
    let result = command_manager
        .execute(0)
        .await
        .expect("Command execution should not fail");
    assert!(result.success);
    assert!(result.output.contains("show version"));

    // And: Attempting to execute with invalid index fails
    let invalid_result = command_manager.execute(10).await;
    assert!(invalid_result.is_err());
    match invalid_result.unwrap_err() {
        embedded_debugger::command::CommandError::InvalidSyntax { line, detail } => {
            assert_eq!(line, 10);
            assert!(detail.contains("not found"));
        }
        _ => panic!("Expected InvalidSyntax error"),
    }

    // Cleanup
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");
}

// CM-006: Multiple commands sequential execution
#[tokio::test]
async fn test_multiple_commands_sequential_execution() {
    // Given: SessionManager and CommandManager are initialized
    let session_manager = SessionManager::new();
    let parser = Box::new(DefaultCommandParser::default());
    let command_manager = DefaultCommandManager::new(parser);

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

    // And: A command file with 10 commands is created
    let temp_dir = create_test_dir();
    let mut command_content = String::new();
    for i in 1..=10 {
        command_content.push_str(&format!("command {}\n", i));
    }
    let command_file = create_test_file(&temp_dir, "sequence.txt", &command_content);

    // When: Commands are loaded
    let count = command_manager
        .load_from_file(&command_file)
        .await
        .expect("Failed to load commands");

    // Then: Exactly 10 commands loaded
    assert_eq!(count, 10);
    assert_eq!(command_manager.count().await, 10);

    // When: Commands are executed sequentially
    let mut execution_results = Vec::new();
    for i in 0..count {
        let result = command_manager
            .execute(i)
            .await
            .expect("Failed to execute command");
        execution_results.push(result);
    }

    // Then: All 10 commands executed successfully
    assert_eq!(execution_results.len(), 10);
    for result in &execution_results {
        assert!(result.success, "Command execution should succeed");
    }

    // And: Each command output matches expected
    for (i, result) in execution_results.iter().enumerate() {
        let expected_command = format!("command {}", i + 1);
        assert!(
            result.output.contains(&expected_command),
            "Expected output to contain '{}', got '{}'",
            expected_command,
            result.output
        );
        assert!(result.error.is_none(), "No errors expected for command {}", i + 1);
    }

    // And: Execution history is tracked
    let history = command_manager.get_execution_history().await;
    assert_eq!(history.len(), 10);
    for (i, (index, result)) in history.iter().enumerate() {
        assert_eq!(*index, i, "Execution index should match");
        assert!(result.success, "History result should be successful");
    }

    // And: Session remains intact
    let session = session_manager
        .get_session(&session_id)
        .await
        .expect("Session not found");
    assert_eq!(session.state(), &embedded_debugger::session::SessionState::Created);
    assert_eq!(session.metadata().name, "test-session");

    // Cleanup
    session_manager
        .close_session(&session_id)
        .await
        .expect("Failed to close session");
}

// Additional test: Command execution history tracking
#[tokio::test]
async fn test_command_execution_history_tracking() {
    // Given: CommandManager with commands
    let parser = Box::new(DefaultCommandParser::default());
    let command_manager = DefaultCommandManager::new(parser);

    command_manager
        .load_from_string("cmd1\ncmd2\ncmd3")
        .await
        .expect("Failed to load commands");

    // When: Commands are executed
    command_manager.execute(0).await.unwrap();
    command_manager.execute(1).await.unwrap();
    command_manager.execute(2).await.unwrap();

    // Then: History contains all executions
    let history = command_manager.get_execution_history().await;
    assert_eq!(history.len(), 3);

    // And: History can be cleared
    command_manager.clear_execution_history().await;
    let cleared_history = command_manager.get_execution_history().await;
    assert!(cleared_history.is_empty());

    // When: New commands are executed
    command_manager.execute(0).await.unwrap();

    // Then: History only contains new execution
    let new_history = command_manager.get_execution_history().await;
    assert_eq!(new_history.len(), 1);
}
