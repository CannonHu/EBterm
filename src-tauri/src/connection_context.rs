//! Connection context for Tauri integration
//!
//! Consolidates all connection-related state: handle, streaming task, stats, and logging

use std::time::Instant;

use ebterm::connection::{ConnectionHandle, ConnectionConfig, ConnectionStats};

use crate::data_streamer::start_batch_streamer;

/// Consolidated connection state
pub struct ConnectionContext {
    /// Connection handle from lib layer
    pub handle: ConnectionHandle,

    /// Data streaming task handle
    pub task_handle: Option<tokio::task::JoinHandle<()>>,

    /// Connection configuration
    pub config: ConnectionConfig,

    /// Connection statistics
    pub stats: ConnectionStats,

    /// Creation timestamp
    pub created_at: Instant,

    /// Optional file logger
    pub logger: Option<ebterm::logger::FileLogger>,
}

impl ConnectionContext {
    pub fn new(handle: ConnectionHandle, config: ConnectionConfig) -> Self {
        Self {
            handle,
            task_handle: None,
            config,
            stats: ConnectionStats::default(),
            created_at: Instant::now(),
            logger: None,
        }
    }

    /// Start background data streaming for this connection
    pub fn start_data_streaming(&mut self, app: tauri::AppHandle, connection_id: String) {
        if self.task_handle.is_some() {
            tracing::warn!(
                connection_id = %connection_id,
                "Data streamer already running for connection"
            );
            return;
        }

        let handle = self.handle.clone();
        let connection_id_clone = connection_id.clone();

        self.task_handle = Some(tokio::spawn(async move {
            start_batch_streamer(connection_id_clone, handle, app).await;
        }));

        tracing::info!(
            connection_id = %connection_id,
            "Data streamer started"
        );
    }

    /// Stop background data streaming
    pub fn stop_data_streaming(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            tracing::info!("Data streamer stopped");
        }
    }

    /// Check if data streaming is active
    pub fn is_streaming(&self) -> bool {
        self.task_handle.is_some()
    }
}

impl std::fmt::Debug for ConnectionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionContext")
            .field("has_handle", &true)
            .field("has_task_handle", &self.task_handle.is_some())
            .field("has_logger", &self.logger.is_some())
            .field("created_at", &self.created_at)
            .finish()
    }
}
