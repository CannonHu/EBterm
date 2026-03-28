//! Tauri application state management
//!
//! Provides shared state for all Tauri commands

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use embedded_debugger::command::manager::DefaultCommandManager;
use embedded_debugger::command::parser::DefaultCommandParser;

use crate::connection_context::ConnectionContext;

/// Application state with consolidated connection management
pub struct AppState {
    /// All connections: id → consolidated context
    pub connections: Arc<RwLock<HashMap<String, ConnectionContext>>>,

    /// Command management
    pub command_manager: Arc<RwLock<DefaultCommandManager>>,
}

impl AppState {
    pub fn new(_app: tauri::AppHandle) -> Self {
        let parser = Box::new(DefaultCommandParser::default());
        let command_manager = Arc::new(RwLock::new(DefaultCommandManager::new(parser)));

        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            command_manager,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        panic!("AppState cannot be created with default() - use AppState::new(app_handle)")
    }
}
