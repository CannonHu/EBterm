//! Tauri application state management
//!
//! Provides shared state for all Tauri commands

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connection_context::ConnectionContext;

/// Application state with consolidated connection management
pub struct AppState {
    /// All connections: id → consolidated context
    pub connections: Arc<RwLock<HashMap<String, ConnectionContext>>>,
}

impl AppState {
    pub fn new(_app: tauri::AppHandle) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        panic!("AppState cannot be created with default() - use AppState::new(app_handle)")
    }
}
