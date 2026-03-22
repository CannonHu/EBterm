//! File logger implementation
//!
//! Provides file-based logging of connection data

use async_trait::async_trait;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Logger, LoggerConfig, LoggerError, LoggerStats};

/// File logger implementation
#[derive(Debug)]
pub struct FileLogger {
    config: LoggerConfig,
    stats: LoggerStats,
    current_file: Option<Arc<Mutex<File>>>,
    current_path: Option<PathBuf>,
    is_logging: bool,
}

impl FileLogger {
    /// Create a new file logger with default configuration
    pub fn new() -> Self {
        Self {
            config: LoggerConfig::default(),
            stats: LoggerStats::default(),
            current_file: None,
            current_path: None,
            is_logging: false,
        }
    }

    /// Create a new file logger with custom configuration
    pub fn with_config(config: LoggerConfig) -> Self {
        Self {
            config,
            stats: LoggerStats::default(),
            current_file: None,
            current_path: None,
            is_logging: false,
        }
    }

    /// Get the current log file path if logging
    pub fn current_log_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    /// Check if currently logging
    pub fn is_logging(&self) -> bool {
        self.is_logging
    }

    /// Rotate log file if needed
    async fn rotate_if_needed(&mut self) -> Result<(), LoggerError> {
        if let Some(ref file) = self.current_file {
            // Check file size
            let file_guard = file.lock().await;
            let metadata = file_guard.metadata()
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
            
            let size = metadata.len();
            drop(file_guard);

            if size >= self.config.max_file_size {
                // Rotate the file
                self.rotate_file().await?;
            }
        }
        
        Ok(())
    }

    /// Perform log file rotation
    async fn rotate_file(&mut self) -> Result<(), LoggerError> {
        if let Some(current_path) = &self.current_path {
            // Close current file
            self.current_file = None;

            // Generate rotated filename
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let stem = current_path.file_stem().unwrap_or_default();
            let ext = current_path.extension().unwrap_or_default();
            let rotated_name = format!("{}_{}.{:?}", stem.to_string_lossy(), timestamp, ext);
            let rotated_path = current_path.with_file_name(rotated_name);

            // Rename current file to rotated name
            std::fs::rename(current_path, &rotated_path)
                .map_err(|e| LoggerError::WriteFailed {
                    path: current_path.display().to_string(),
                    reason: format!("Failed to rotate file: {}", e),
                })?;

            // Reopen new log file
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(current_path)
                .map_err(|e| LoggerError::OpenFailed {
                    path: current_path.display().to_string(),
                    reason: e.to_string(),
                })?;

            self.current_file = Some(Arc::new(Mutex::new(file)));
        }

        Ok(())
    }
}

impl Default for FileLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Logger for FileLogger {
    async fn start_logging(&mut self, file_path: &Path) -> Result<(), LoggerError> {
        if self.is_logging {
            return Err(LoggerError::AlreadyOpen {
                path: self.current_path.as_ref().unwrap().display().to_string(),
            });
        }

        // Create/open the log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| LoggerError::OpenFailed {
                path: file_path.display().to_string(),
                reason: e.to_string(),
            })?;

        self.current_file = Some(Arc::new(Mutex::new(file)));
        self.current_path = Some(file_path.to_path_buf());
        self.is_logging = true;
        self.stats.started_at = Some(std::time::SystemTime::now());

        Ok(())
    }

    async fn stop_logging(&mut self) -> Result<(), LoggerError> {
        if !self.is_logging {
            return Err(LoggerError::NotOpen {
                path: "unknown".to_string(),
            });
        }

        // Flush and close the file
        if let Some(file) = self.current_file.take() {
            let mut file_guard = file.lock().await;
            file_guard.flush()
                .map_err(|e| LoggerError::CloseFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
        }

        self.current_path = None;
        self.is_logging = false;

        Ok(())
    }

    async fn log_input(&mut self, _session_id: &str, data: &[u8]) -> Result<(), LoggerError> {
        if !self.is_logging {
            return Err(LoggerError::NotEnabled {
                session_id: _session_id.to_string(),
            });
        }

        // Check if rotation is needed
        self.rotate_if_needed().await?;

        // Write to log file
        if let Some(file) = &self.current_file {
            let mut file_guard = file.lock().await;
            
            // Write timestamp and direction
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let header = format!("[{}] [INPUT] ", timestamp);
            file_guard.write_all(header.as_bytes())
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
            
            // Write data (as hex for binary data)
            for byte in data {
                write!(file_guard, "{:02x}", byte)
                    .map_err(|e| LoggerError::WriteFailed {
                        path: self.current_path.as_ref().unwrap().display().to_string(),
                        reason: e.to_string(),
                    })?;
            }
            
            file_guard.write_all(b"\n")
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
        }

        self.stats.bytes_logged_input += data.len() as u64;
        self.stats.total_entries += 1;

        Ok(())
    }

    async fn log_output(&mut self, _session_id: &str, data: &[u8]) -> Result<(), LoggerError> {
        if !self.is_logging {
            return Err(LoggerError::NotEnabled {
                session_id: _session_id.to_string(),
            });
        }

        // Check if rotation is needed
        self.rotate_if_needed().await?;

        // Write to log file
        if let Some(file) = &self.current_file {
            let mut file_guard = file.lock().await;
            
            // Write timestamp and direction
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let header = format!("[{}] [OUTPUT] ", timestamp);
            file_guard.write_all(header.as_bytes())
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
            
            // Write data (as hex for binary data)
            for byte in data {
                write!(file_guard, "{:02x}", byte)
                    .map_err(|e| LoggerError::WriteFailed {
                        path: self.current_path.as_ref().unwrap().display().to_string(),
                        reason: e.to_string(),
                    })?;
            }
            
            file_guard.write_all(b"\n")
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
        }

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
        if let Some(file) = &self.current_file {
            let mut file_guard = file.lock().await;
            file_guard.flush()
                .map_err(|e| LoggerError::WriteFailed {
                    path: self.current_path.as_ref().unwrap().display().to_string(),
                    reason: e.to_string(),
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_file_logger_creation() {
        let logger = FileLogger::new();
        assert!(!logger.is_logging());
        assert!(logger.current_log_path().is_none());
    }

    #[test]
    fn test_file_logger_with_config() {
        let config = LoggerConfig {
            max_file_size: 5 * 1024 * 1024, // 5MB
            max_backup_files: 3,
            compress_rotated: false,
            buffer_size: 4096,
        };
        
        let logger = FileLogger::with_config(config.clone());
        assert_eq!(logger.config().max_file_size, 5 * 1024 * 1024);
        assert_eq!(logger.config().max_backup_files, 3);
        assert!(!logger.config().compress_rotated);
    }

    #[tokio::test]
    async fn test_file_logger_start_stop() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let mut logger = FileLogger::new();
        
        // Start logging
        logger.start_logging(&log_path).await.expect("Failed to start logging");
        assert!(logger.is_logging());
        assert_eq!(logger.current_log_path(), Some(log_path.as_path()));
        
        // Stop logging
        logger.stop_logging().await.expect("Failed to stop logging");
        assert!(!logger.is_logging());
        assert!(logger.current_log_path().is_none());
    }

    #[tokio::test]
    async fn test_file_logger_double_start() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let mut logger = FileLogger::new();
        
        // Start logging
        logger.start_logging(&log_path).await.expect("Failed to start logging");
        
        // Try to start again (should fail)
        let result = logger.start_logging(&log_path).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoggerError::AlreadyOpen { .. }));
    }

    #[tokio::test]
    async fn test_file_logger_log_input_output() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        let mut logger = FileLogger::new();
        
        // Start logging
        logger.start_logging(&log_path).await.expect("Failed to start logging");
        
        // Log some input data
        let input_data = b"Hello, World!";
        logger.log_input("session-1", input_data).await.expect("Failed to log input");
        
        // Log some output data
        let output_data = b"Response data";
        logger.log_output("session-1", output_data).await.expect("Failed to log output");
        
        // Stop logging
        logger.stop_logging().await.expect("Failed to stop logging");
        
        // Verify stats
        let stats = logger.stats();
        assert_eq!(stats.bytes_logged_input, input_data.len() as u64);
        assert_eq!(stats.bytes_logged_output, output_data.len() as u64);
        assert_eq!(stats.total_entries, 2);
        
        // Verify log file contents
        let mut file = File::open(&log_path).expect("Failed to open log file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read log file");
        
        // Check that both input and output are logged
        assert!(contents.contains("[INPUT]"));
        assert!(contents.contains("[OUTPUT]"));
    }

    #[tokio::test]
    async fn test_file_logger_log_when_not_logging() {
        let mut logger = FileLogger::new();
        
        // Try to log when not started
        let result = logger.log_input("session-1", b"test").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoggerError::NotEnabled { .. }));
    }

    #[test]
    fn test_file_logger_stats() {
        let mut logger = FileLogger::new();
        
        let stats = logger.stats();
        assert_eq!(stats.bytes_logged_input, 0);
        assert_eq!(stats.bytes_logged_output, 0);
        assert_eq!(stats.total_entries, 0);
        assert!(stats.started_at.is_none());
        
        // Test clear_stats
        logger.clear_stats();
        let stats_after_clear = logger.stats();
        assert_eq!(stats_after_clear.bytes_logged_input, 0);
        assert_eq!(stats_after_clear.bytes_logged_output, 0);
    }

    #[test]
    fn test_file_logger_config() {
        let mut logger = FileLogger::new();
        
        let config = logger.config();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024); // 10MB default
        
        let new_config = LoggerConfig {
            max_file_size: 5 * 1024 * 1024,
            max_backup_files: 3,
            compress_rotated: false,
            buffer_size: 4096,
        };
        
        logger.set_config(new_config.clone());
        assert_eq!(logger.config().max_file_size, 5 * 1024 * 1024);
        assert_eq!(logger.config().max_backup_files, 3);
    }
}