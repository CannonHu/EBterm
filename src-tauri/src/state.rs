//! Tauri application state management
//!
//! Provides shared state for all Tauri commands

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use embedded_debugger::command::manager::DefaultCommandManager;
use embedded_debugger::command::parser::DefaultCommandParser;
use embedded_debugger::logger::FileLogger;
use embedded_debugger::session::manager::SessionManager;

pub struct AppState {
    pub session_manager: Arc<RwLock<SessionManager>>,
    pub command_manager: Arc<RwLock<DefaultCommandManager>>,
    pub loggers: Arc<RwLock<HashMap<String, FileLogger>>>,
}

impl AppState {
    pub fn new() -> Self {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));

        let parser = Box::new(DefaultCommandParser::default());
        let command_manager = Arc::new(RwLock::new(DefaultCommandManager::new(parser)));

        Self {
            session_manager,
            command_manager,
            loggers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// AppState uses Arc<RwLock<...>> for all internal state, which is already Send + Sync
// The unsafe impl is not needed since all components are properly thread-safe
