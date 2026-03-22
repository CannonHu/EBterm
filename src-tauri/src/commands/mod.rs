//! Tauri IPC commands for embedded-debugger
//!
//! Provides command handlers for frontend-to-backend communication

pub mod connection;
pub mod session;
pub mod logging;
pub mod command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: serde::Serialize> CommandResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

pub type CommandResponse<T> = Result<CommandResult<T>, String>;

pub fn ok<T: serde::Serialize>(data: T) -> CommandResponse<T> {
    Ok(CommandResult::success(data))
}

pub fn err<T: serde::Serialize>(message: impl Into<String>) -> CommandResponse<T> {
    Ok(CommandResult::error(message))
}