use async_trait::async_trait;
use std::path::Path;

/// Log direction - represents data flow direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogDirection {
    /// Data received from connection (input)
    Input,
    /// Data sent to connection (output)
    Output,
}

impl std::fmt::Display for LogDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogDirection::Input => write!(f, "Input"),
            LogDirection::Output => write!(f, "Output"),
        }
    }
}

/// Log entry metadata
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp when the data was logged
    pub timestamp: std::time::SystemTime,
    /// Direction of data flow
    pub direction: LogDirection,
    /// The raw data bytes
    pub data: Vec<u8>,
    /// Session ID that generated this log
    pub session_id: String,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        session_id: impl Into<String>,
        direction: LogDirection,
        data: Vec<u8>,
    ) -> Self {
        Self {
            timestamp: std::time::SystemTime::now(),
            direction,
            data,
            session_id: session_id.into(),
        }
    }

    /// Get the data as a hex string
    pub fn data_as_hex(&self) -> String {
        self.data.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Get the data as a string (lossy conversion)
    pub fn data_as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

/// Logger statistics
#[derive(Debug, Clone, Default)]
pub struct LoggerStats {
    /// Total bytes logged for input
    pub bytes_logged_input: u64,
    /// Total bytes logged for output
    pub bytes_logged_output: u64,
    /// Total number of log entries
    pub total_entries: u64,
    /// When logging started
    pub started_at: Option<std::time::SystemTime>,
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// Maximum file size in bytes before rotation
    pub max_file_size: u64,
    /// Maximum number of backup files to keep
    pub max_backup_files: u32,
    /// Whether to compress rotated files
    pub compress_rotated: bool,
    /// Buffer size for writes
    pub buffer_size: usize,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_backup_files: 5,
            compress_rotated: true,
            buffer_size: 8192, // 8KB
        }
    }
}

/// Core logger trait
#[async_trait]
pub trait Logger: Send + Sync {
    /// Start logging to a file
    async fn start_logging(&mut self, file_path: &Path) -> Result<(), LoggerError>;
    
    /// Stop logging
    async fn stop_logging(&mut self) -> Result<(), LoggerError>;
    
    /// Log input data (received from connection)
    async fn log_input(&mut self, session_id: &str, data: &[u8]) -> Result<(), LoggerError>;
    
    /// Log output data (sent to connection)
    async fn log_output(&mut self, session_id: &str, data: &[u8]) -> Result<(), LoggerError>;
    
    /// Check if currently logging
    fn is_logging(&self) -> bool;
    
    /// Get current log file path if any
    fn current_log_path(&self) -> Option<&Path>;
    
    /// Get logger statistics
    fn stats(&self) -> LoggerStats;
    
    /// Clear statistics
    fn clear_stats(&mut self);
    
    /// Get logger configuration
    fn config(&self) -> &LoggerConfig;
    
    /// Update logger configuration
    fn set_config(&mut self, config: LoggerConfig);
    
    /// Flush any buffered data to disk
    async fn flush(&mut self) -> Result<(), LoggerError>;
}

use super::LoggerError;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct MockLogger {
        is_logging: bool,
        current_path: Option<PathBuf>,
        stats: LoggerStats,
        config: LoggerConfig,
    }

    impl MockLogger {
        fn new() -> Self {
            Self {
                is_logging: false,
                current_path: None,
                stats: LoggerStats::default(),
                config: LoggerConfig::default(),
            }
        }
    }

    #[async_trait::async_trait]
    impl Logger for MockLogger {
        async fn start_logging(&mut self, file_path: &Path) -> Result<(), LoggerError> {
            self.is_logging = true;
            self.current_path = Some(file_path.to_path_buf());
            self.stats.started_at = Some(std::time::SystemTime::now());
            Ok(())
        }

        async fn stop_logging(&mut self) -> Result<(), LoggerError> {
            self.is_logging = false;
            self.current_path = None;
            Ok(())
        }

        async fn log_input(&mut self, _session_id: &str, data: &[u8]) -> Result<(), LoggerError> {
            self.stats.bytes_logged_input += data.len() as u64;
            self.stats.total_entries += 1;
            Ok(())
        }

        async fn log_output(&mut self, _session_id: &str, data: &[u8]) -> Result<(), LoggerError> {
            self.stats.bytes_logged_output += data.len() as u64;
            self.stats.total_entries += 1;
            Ok(())
        }

        fn is_logging(&self) -> bool {
            self.is_logging
        }

        fn current_log_path(&self) -> Option<&Path> {
            self.current_path.as_deref()
        }

        fn stats(&self) -> LoggerStats {
            self.stats.clone()
        }

        fn clear_stats(&mut self) {
            self.stats = LoggerStats::default();
        }

        fn config(&self) -> &LoggerConfig {
            &self.config
        }

        fn set_config(&mut self, config: LoggerConfig) {
            self.config = config;
        }

        async fn flush(&mut self) -> Result<(), LoggerError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_logger_lifecycle() {
        let mut logger = MockLogger::new();
        
        assert!(!logger.is_logging());
        assert!(logger.current_log_path().is_none());
        
        // Start logging
        logger.start_logging(Path::new("/tmp/test.log")).await.unwrap();
        assert!(logger.is_logging());
        assert_eq!(logger.current_log_path(), Some(Path::new("/tmp/test.log")));
        
        // Stop logging
        logger.stop_logging().await.unwrap();
        assert!(!logger.is_logging());
        assert!(logger.current_log_path().is_none());
    }

    #[tokio::test]
    async fn test_mock_logger_log_input() {
        let mut logger = MockLogger::new();
        logger.start_logging(Path::new("/tmp/test.log")).await.unwrap();
        
        let data = b"Hello, World!";
        logger.log_input("session-1", data).await.unwrap();
        
        let stats = logger.stats();
        assert_eq!(stats.bytes_logged_input, data.len() as u64);
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_mock_logger_log_output() {
        let mut logger = MockLogger::new();
        logger.start_logging(Path::new("/tmp/test.log")).await.unwrap();
        
        let data = b"Response data";
        logger.log_output("session-1", data).await.unwrap();
        
        let stats = logger.stats();
        assert_eq!(stats.bytes_logged_output, data.len() as u64);
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_mock_logger_clear_stats() {
        let mut logger = MockLogger::new();
        logger.start_logging(Path::new("/tmp/test.log")).await.unwrap();
        
        logger.log_input("session-1", b"test").await.unwrap();
        logger.log_output("session-1", b"test").await.unwrap();
        
        let stats = logger.stats();
        assert!(stats.total_entries > 0);
        
        logger.clear_stats();
        let stats = logger.stats();
        assert_eq!(stats.bytes_logged_input, 0);
        assert_eq!(stats.bytes_logged_output, 0);
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_mock_logger_config() {
        let mut logger = MockLogger::new();
        
        let config = logger.config();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024); // 10MB default
        
        let new_config = LoggerConfig {
            max_file_size: 5 * 1024 * 1024, // 5MB
            max_backup_files: 3,
            compress_rotated: false,
            buffer_size: 4096,
        };
        
        logger.set_config(new_config.clone());
        assert_eq!(logger.config().max_file_size, 5 * 1024 * 1024);
        assert_eq!(logger.config().max_backup_files, 3);
    }

    #[tokio::test]
    async fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024); // 10MB
        assert_eq!(config.max_backup_files, 5);
        assert!(config.compress_rotated);
        assert_eq!(config.buffer_size, 8192); // 8KB
    }

    #[tokio::test]
    async fn test_logger_stats_default() {
        let stats = LoggerStats::default();
        assert_eq!(stats.bytes_logged_input, 0);
        assert_eq!(stats.bytes_logged_output, 0);
        assert_eq!(stats.total_entries, 0);
        assert!(stats.started_at.is_none());
    }

    #[tokio::test]
    async fn test_log_entry_creation() {
        let entry = LogEntry::new(
            "session-123",
            LogDirection::Input,
            b"Hello, World!".to_vec(),
        );
        
        assert_eq!(entry.session_id, "session-123");
        assert_eq!(entry.direction, LogDirection::Input);
        assert_eq!(entry.data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_log_entry_data_as_hex() {
        let entry = LogEntry::new(
            "session-123",
            LogDirection::Output,
            vec![0x48, 0x65, 0x6c, 0x6c, 0x6f], // "Hello"
        );
        
        assert_eq!(entry.data_as_hex(), "48656c6c6f");
    }

    #[tokio::test]
    async fn test_log_entry_data_as_string() {
        let entry = LogEntry::new(
            "session-123",
            LogDirection::Input,
            b"Hello, World!".to_vec(),
        );
        
        assert_eq!(entry.data_as_string(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_log_direction_display() {
        assert_eq!(LogDirection::Input.to_string(), "Input");
        assert_eq!(LogDirection::Output.to_string(), "Output");
    }

    #[tokio::test]
    async fn test_mock_logger_flush() {
        let mut logger = MockLogger::new();
        logger.start_logging(Path::new("/tmp/test.log")).await.unwrap();
        logger.log_input("session-1", b"test data").await.unwrap();

        let flush_result = logger.flush().await;
        assert!(flush_result.is_ok());
    }
}
