//! Command manager for executing and managing commands
//!
//! Provides functionality for managing loaded commands and executing them

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::parser::{CommandParser, ParsedCommand};
use super::CommandError;

/// Command execution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Output/response from the command
    pub output: String,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ExecutionResult {
    /// Create a successful result
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            execution_time_ms: 0,
        }
    }

    /// Create a failed result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
            execution_time_ms: 0,
        }
    }
}

/// Trait for managing and executing commands
#[async_trait::async_trait]
pub trait CommandManager: Send + Sync {
    /// Load commands from a file
    async fn load_from_file(&self, path: &Path) -> Result<usize, CommandError>;

    /// Load commands from a string
    async fn load_from_string(&self, content: &str) -> Result<usize, CommandError>;

    /// Get the currently loaded commands
    async fn get_commands(&self) -> Vec<ParsedCommand>;

    /// Get a specific command by index
    async fn get_command(&self, index: usize) -> Option<ParsedCommand>;

    /// Execute a command by index
    async fn execute(&self, index: usize) -> Result<ExecutionResult, CommandError>;

    /// Execute a command by its content
    async fn execute_command(&self, command: &str) -> Result<ExecutionResult, CommandError>;

    /// Clear all loaded commands
    async fn clear(&self);

    /// Get the number of loaded commands
    async fn count(&self) -> usize;

    /// Check if commands are loaded
    async fn is_empty(&self) -> bool;
}

/// Default command manager implementation
pub struct DefaultCommandManager {
    parser: Box<dyn CommandParser>,
    commands: Arc<RwLock<Vec<ParsedCommand>>>,
    execution_history: Arc<RwLock<Vec<(usize, ExecutionResult)>>>,
}

impl DefaultCommandManager {
    /// Create a new command manager with the given parser
    pub fn new(parser: Box<dyn CommandParser>) -> Self {
        Self {
            parser,
            commands: Arc::new(RwLock::new(Vec::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get execution history
    pub async fn get_execution_history(&self) -> Vec<(usize, ExecutionResult)> {
        self.execution_history.read().await.clone()
    }

    /// Clear execution history
    pub async fn clear_execution_history(&self) {
        self.execution_history.write().await.clear();
    }
}

#[async_trait::async_trait]
impl CommandManager for DefaultCommandManager {
    async fn load_from_file(&self, path: &Path) -> Result<usize, CommandError> {
        let commands = self.parser.parse_file(path)?;
        let count = commands.len();
        *self.commands.write().await = commands;
        Ok(count)
    }

    async fn load_from_string(&self, content: &str) -> Result<usize, CommandError> {
        let commands = self.parser.parse_string(content)?;
        let count = commands.len();
        *self.commands.write().await = commands;
        Ok(count)
    }

    async fn get_commands(&self) -> Vec<ParsedCommand> {
        self.commands.read().await.clone()
    }

    async fn get_command(&self, index: usize) -> Option<ParsedCommand> {
        self.commands.read().await.get(index).cloned()
    }

    async fn execute(&self, index: usize) -> Result<ExecutionResult, CommandError> {
        let command = self
            .get_command(index)
            .await
            .ok_or_else(|| CommandError::InvalidSyntax {
                line: index,
                detail: format!("Command at index {} not found", index),
            })?;

        let result = self.execute_command(&command.content).await?;
        self.execution_history
            .write()
            .await
            .push((index, result.clone()));

        Ok(result)
    }

    async fn execute_command(&self, command: &str) -> Result<ExecutionResult, CommandError> {
        let start = std::time::Instant::now();

        // For now, simulate command execution
        // In a real implementation, this would send the command to the device
        let result = if command.trim().is_empty() {
            ExecutionResult::failure("Empty command")
        } else {
            let mut success = ExecutionResult::success(format!("Executed: {}", command));
            success.execution_time_ms = start.elapsed().as_millis() as u64;
            success
        };

        Ok(result)
    }

    async fn clear(&self) {
        self.commands.write().await.clear();
    }

    async fn count(&self) -> usize {
        self.commands.read().await.len()
    }

    async fn is_empty(&self) -> bool {
        self.commands.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::success("Output text");
        assert!(result.success);
        assert_eq!(result.output, "Output text");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::failure("Error message");
        assert!(!result.success);
        assert_eq!(result.output, "");
        assert_eq!(result.error, Some("Error message".to_string()));
    }

    #[test]
    fn test_execution_result_with_timing() {
        let mut result = ExecutionResult::success("Done");
        result.execution_time_ms = 150;
        assert_eq!(result.execution_time_ms, 150);
    }

    // Note: These tests require async runtime and are tested in integration tests
    // The following are compile-time checks to ensure the traits are implemented correctly

    #[tokio::test]
    async fn test_default_command_manager_creation() {
        use crate::command::parser::DefaultCommandParser;

        let parser = Box::new(DefaultCommandParser::default());
        let _manager = DefaultCommandManager::new(parser);
        // If we get here, the manager was created successfully
    }

    #[tokio::test]
    async fn test_command_manager_load_from_string() {
        use crate::command::parser::DefaultCommandParser;
        use crate::command::manager::CommandManager;

        let parser = Box::new(DefaultCommandParser::default());
        let manager = DefaultCommandManager::new(parser);

        let content = "show version\nshow running-config";
        let count = manager.load_from_string(content).await.unwrap();

        assert_eq!(count, 2);
        assert_eq!(manager.count().await, 2);
    }

    #[tokio::test]
    async fn test_command_manager_get_commands() {
        use crate::command::parser::DefaultCommandParser;
        use crate::command::manager::CommandManager;

        let parser = Box::new(DefaultCommandParser::default());
        let manager = DefaultCommandManager::new(parser);

        let content = "show version\nshow running-config";
        manager.load_from_string(content).await.unwrap();

        let commands = manager.get_commands().await;
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].content, "show version");
        assert_eq!(commands[1].content, "show running-config");
    }

    #[tokio::test]
    async fn test_command_manager_execute_command() {
        use crate::command::parser::DefaultCommandParser;
        use crate::command::manager::CommandManager;

        let parser = Box::new(DefaultCommandParser::default());
        let manager = DefaultCommandManager::new(parser);

        let result = manager.execute_command("show version").await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("Executed: show version"));
    }

    #[tokio::test]
    async fn test_command_manager_clear() {
        use crate::command::parser::DefaultCommandParser;
        use crate::command::manager::CommandManager;

        let parser = Box::new(DefaultCommandParser::default());
        let manager = DefaultCommandManager::new(parser);

        let content = "show version\nshow running-config";
        manager.load_from_string(content).await.unwrap();
        assert_eq!(manager.count().await, 2);

        manager.clear().await;
        assert_eq!(manager.count().await, 0);
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_command_manager_is_empty() {
        use crate::command::parser::DefaultCommandParser;
        use crate::command::manager::CommandManager;

        let parser = Box::new(DefaultCommandParser::default());
        let manager = DefaultCommandManager::new(parser);

        assert!(manager.is_empty().await);

        manager.load_from_string("show version").await.unwrap();
        assert!(!manager.is_empty().await);
    }
}