//! File logger implementation
//!
//! Provides simple file-based logging of plain text content.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use regex::Regex;

use super::LoggerError;

/// Regex to match all ANSI escape sequences
static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\x1B(?:[@-Z\\-_]|\[.*?[a-zA-Z])").unwrap());

/// Simple file logger that writes plain text to a file
#[derive(Debug)]
pub struct FileLogger {
    file: Option<Mutex<File>>,
}

impl FileLogger {
    pub fn new() -> Self {
        Self { file: None }
    }

    pub fn start(&mut self, path: &Path) -> Result<(), LoggerError> {
        if self.file.is_some() {
            return Err(LoggerError::WriteFailed(
                "Logger already started".to_string(),
            ));
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| {
                LoggerError::OpenFailed(format!("Failed to open file {}: {}", path.display(), e))
            })?;

        self.file = Some(Mutex::new(file));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), LoggerError> {
        let file = self.file.take();
        if let Some(file) = file {
            let mut file_guard = file.lock().unwrap();
            file_guard
                .flush()
                .map_err(|e| LoggerError::WriteFailed(format!("Failed to flush file: {}", e)))?;
        }
        Ok(())
    }

    pub fn write(&self, text: &str) -> Result<(), LoggerError> {
        if let Some(file) = &self.file {
            let mut file_guard = file.lock().unwrap();

            // Step 1: Remove all ANSI escape sequences
            let filtered_ansi = ANSI_REGEX.replace_all(text, "");

            // Step 2: Remove control characters except newline, carriage return, tab
            let filtered_text: String = filtered_ansi.chars()
                .filter(|c| {
                    !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t'
                })
                .collect();

            file_guard
                .write_all(filtered_text.as_bytes())
                .map_err(|e| LoggerError::WriteFailed(format!("Failed to write data: {}", e)))?;
        } else {
            return Err(LoggerError::NotStarted);
        }
        Ok(())
    }
}

impl Default for FileLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    #[test]
    fn test_file_logger_creation() {
        let logger = FileLogger::new();
    }

    #[test]
    fn test_file_logger_default() {
        let logger = FileLogger::default();
    }

    #[test]
    fn test_file_logger_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        assert!(logger.start(&log_path).is_ok());
        assert!(logger.stop().is_ok());
    }

    #[test]
    fn test_file_logger_double_start() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        assert!(logger.start(&log_path).is_ok());
        assert!(logger.start(&log_path).is_err());
    }

    #[test]
    fn test_file_logger_write_text() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        logger.write("Hello, World!").unwrap();
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "Hello, World!");
    }

    #[test]
    fn test_file_logger_write_multiline() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        logger.write("Line 1\nLine 2\nLine 3").unwrap();
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_file_logger_write_not_started() {
        let logger = FileLogger::new();
        let result = logger.write("test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoggerError::NotStarted));
    }

    #[test]
    fn test_file_logger_stop_not_started() {
        let mut logger = FileLogger::new();
        let result = logger.stop();
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_logger_append() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        logger.write("First line\n").unwrap();
        logger.stop().unwrap();

        let mut logger2 = FileLogger::new();
        logger2.start(&log_path).unwrap();
        logger2.write("Second line\n").unwrap();
        logger2.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "First line\nSecond line\n");
    }

    #[test]
    fn test_file_logger_filter_ansi_sequences() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        // Test various ANSI sequences
        logger.write("\x1b[1mBold text\x1b[0m\n").unwrap();
        logger.write("\x1b[31mRed text\x1b[0m\n").unwrap();
        logger.write("\x1b[2JClear screen\x1b[H\n").unwrap();
        logger.write("\x1b[A Move up\n").unwrap();
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "Bold text\nRed text\nClear screen\n Move up\n");
    }

    #[test]
    fn test_file_logger_filter_control_chars() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        // Test control characters
        logger.write("Normal text\x7F with backspace\n").unwrap(); // DEL character
        logger.write("Text\x00 with null byte\n").unwrap();
        logger.write("Text\x08 with backspace\x08\n").unwrap(); // BS character
        logger.write("Line1\nLine2\tindented\rLine3\n").unwrap(); // Keep \n, \t, \r
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "Normal tex with backspace\nText with null byte\nTex with backspac\nLine1\nLine2\tindented\rLine3\n");
    }

    #[test]
    fn test_file_logger_preserve_percent_sign() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        logger.write("Progress: 50% complete\n").unwrap();
        logger.write("100% finished\n").unwrap();
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "Progress: 50% complete\n100% finished\n");
    }

    #[test]
    fn test_file_logger_mixed_content() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        logger.write("\x1b[32m[INFO]\x1b[0m Download progress: 75%\x7F\x7F80%\n").unwrap();
        logger.write("\x1b[1;31mERROR\x1b[0m: Connection failed\x00\n").unwrap();
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "[INFO] Download progress: 780%\nERROR: Connection failed\n");
    }

    #[test]
    fn test_file_logger_backspace_processing() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        // Test basic backspace scenario mentioned by user
        logger.write("lss\x7F\n").unwrap(); // lss + backspace → ls
        logger.write("test\x08\x08ing\n").unwrap(); // test + 2 backspace + ing → ting
        logger.write("a\x7F\x7F\x7Fbb\n").unwrap(); // a + 3 backspace (one beyond start) + bb → bb
        logger.write("\x7F\x7Fprefix\n").unwrap(); // leading backspaces are ignored
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "ls\nteing\nbb\nprefix\n");
    }

    #[test]
    fn test_file_logger_backspace_with_ansi() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut logger = FileLogger::new();
        logger.start(&log_path).unwrap();
        // ANSI sequence + backspace combination
        logger.write("\x1b[32mlss\x7F\x1b[0m\n").unwrap(); // green lss + backspace → green ls
        logger.write("com\x1b[1mman\x7Fd\x1b[0m\n").unwrap(); // com + bold man + backspace + d → command
        logger.stop().unwrap();

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert_eq!(contents, "ls\ncommad\n");
    }
}
