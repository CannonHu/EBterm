//! Tauri application state management
//!
//! Provides shared state for all Tauri commands

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use embedded_debugger::command::manager::DefaultCommandManager;
use embedded_debugger::command::parser::DefaultCommandParser;
use embedded_debugger::connection::ConnectionManager;
use embedded_debugger::logger::FileLogger;

use crate::data_streamer::DataStreamerManager;

pub struct AppState {
    pub connection_manager: Arc<RwLock<ConnectionManager>>,
    pub command_manager: Arc<RwLock<DefaultCommandManager>>,
    pub loggers: Arc<RwLock<HashMap<String, FileLogger>>>,
    pub data_streamer_manager: Arc<DataStreamerManager>,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> Self {
        let data_streamer_manager = Arc::new(DataStreamerManager::new(app));

        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new()));

        let parser = Box::new(DefaultCommandParser::default());
        let command_manager = Arc::new(RwLock::new(DefaultCommandManager::new(parser)));

        Self {
            connection_manager,
            command_manager,
            loggers: Arc::new(RwLock::new(HashMap::new())),
            data_streamer_manager,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        panic!("AppState cannot be created with default() - use AppState::new(app_handle)")
    }
}
