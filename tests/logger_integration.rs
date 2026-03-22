use embedded_debugger::logger::{FileLogger, Logger};
use tokio;

mod common;
use common::*;

#[tokio::test]
async fn test_logger_file_creation() {
    // Given: a temporary directory and a log file path that does not exist
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("test.log");
    assert!(!log_file.exists());

    // When: creating a FileLogger and starting logging to the file path
    let mut logger = FileLogger::new();
    logger.start_logging(&log_file).await.expect("Failed to start logging");

    // Then: the log file should exist on the filesystem
    assert!(log_file.exists());
}

#[tokio::test]
async fn test_logger_input_logging() {
    // Given: a temporary directory, a log file path, and test input data
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("input_test.log");

    let mut logger = FileLogger::new();
    logger.start_logging(&log_file).await.expect("Failed to start logging");

    // When: logging input data from a session and stopping the logger
    let test_data = b"Test input data from device";
    logger.log_input("session-001", test_data).await.expect("Failed to log input");
    logger.stop_logging().await.expect("Failed to stop logging");

    // Then: the log file should exist and contain logged content
    assert!(log_file.exists());
    let content = read_file(&log_file);
    assert!(!content.is_empty());
}

#[tokio::test]
async fn test_logger_output_logging() {
    // Given: a temporary directory, a log file path, and test output data
    let temp_dir = create_test_dir();
    let log_file = temp_dir.path().join("output_test.log");

    let mut logger = FileLogger::new();
    logger.start_logging(&log_file).await.expect("Failed to start logging");

    // When: logging output data to a session and stopping the logger
    let test_data = b"Test output data to device";
    logger.log_output("session-001", test_data).await.expect("Failed to log output");
    logger.stop_logging().await.expect("Failed to stop logging");

    // Then: the log file should exist and contain logged content
    assert!(log_file.exists());
    let content = read_file(&log_file);
    assert!(!content.is_empty());
}
