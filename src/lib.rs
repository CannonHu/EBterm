//! # Embedded Debugger
//!
//! Cross-platform embedded board debugging tool with support for:
//! - Serial port communication (via serial2-tokio)
//! - Telnet connections (via raw TCP)
//! - Tauri GUI framework
//! - Command file parsing
//! - Real-time logging

pub mod connection;
pub mod logger;
pub mod error;

pub use error::{Error, Result};
