use embedded_debugger::logger::FileLogger;
use std::io::Read;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

fn get_test_file_path(dir: &TempDir, filename: &str) -> PathBuf {
    dir.path().join(filename)
}

#[tokio::test]
async fn test_logger_file_creation() {
    let temp_dir = create_test_dir();
    let log_file = get_test_file_path(&temp_dir, "test.log");
    assert!(!log_file.exists());

    let mut logger = FileLogger::new();
    logger.start(&log_file).expect("Failed to start logging");
    assert!(log_file.exists());
    logger.stop().expect("Failed to stop logging");
}

#[tokio::test]
async fn test_logger_write_text() {
    let temp_dir = create_test_dir();
    let log_file = get_test_file_path(&temp_dir, "test.log");

    let mut logger = FileLogger::new();
    logger.start(&log_file).expect("Failed to start logging");
    logger.write("Hello, World!").expect("Failed to write data");
    logger.stop().expect("Failed to stop logging");

    let mut file = std::fs::File::open(&log_file).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "Hello, World!");
}

#[tokio::test]
async fn test_logger_write_multiline() {
    let temp_dir = create_test_dir();
    let log_file = get_test_file_path(&temp_dir, "test.log");

    let mut logger = FileLogger::new();
    logger.start(&log_file).expect("Failed to start logging");

    logger.write("Line 1\nLine 2\nLine 3").expect("Failed to write data");
    logger.stop().expect("Failed to stop logging");

    let mut file = std::fs::File::open(&log_file).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "Line 1\nLine 2\nLine 3");
}

#[tokio::test]
async fn test_logger_write_not_started() {
    let logger = FileLogger::new();
    let result = logger.write("test");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), embedded_debugger::logger::LoggerError::NotStarted));
}

#[tokio::test]
async fn test_logger_double_start() {
    let temp_dir = create_test_dir();
    let log_file = get_test_file_path(&temp_dir, "test.log");

    let mut logger = FileLogger::new();
    logger.start(&log_file).expect("Failed to start logging");
    assert!(logger.start(&log_file).is_err());
}

#[tokio::test]
async fn test_logger_stop_not_started() {
    let mut logger = FileLogger::new();
    let result = logger.stop();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_logger_append() {
    let temp_dir = create_test_dir();
    let log_file = get_test_file_path(&temp_dir, "test.log");

    let mut logger1 = FileLogger::new();
    logger1.start(&log_file).expect("Failed to start logging");
    logger1.write("First line\n").expect("Failed to write data");
    logger1.stop().expect("Failed to stop logging");

    let mut logger2 = FileLogger::new();
    logger2.start(&log_file).expect("Failed to start logging");
    logger2.write("Second line\n").expect("Failed to write data");
    logger2.stop().expect("Failed to stop logging");

    let mut file = std::fs::File::open(&log_file).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "First line\nSecond line\n");
}
