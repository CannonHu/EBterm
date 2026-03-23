//! Data streaming module for Tauri integration
//!
//! Manages background read loops for connected sessions, batching data
//! and emitting Tauri events to the frontend.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, broadcast};
use bytes::BytesMut;
use tauri::Manager;

use embedded_debugger::session::ConnectionRegistry;
use embedded_debugger::session::SessionId;

use crate::ipc::DataReceivedEvent;

/// Configuration for data streaming behavior
#[derive(Debug, Clone)]
pub struct DataStreamerConfig {
    /// Minimum delay before flushing (ms) - used for low throughput
    pub min_batch_delay_ms: u64,
    /// Maximum delay before forcing flush (ms)
    pub max_batch_delay_ms: u64,
    /// Batch size threshold (bytes) - used for high throughput
    pub batch_size: usize,
    /// Read buffer size (bytes)
    pub read_buffer_size: usize,
}

impl Default for DataStreamerConfig {
    fn default() -> Self {
        Self {
            min_batch_delay_ms: 1,
            max_batch_delay_ms: 16,
            batch_size: 4096,
            read_buffer_size: 16384,
        }
    }
}

/// Manages data streaming for a single session
pub struct DataStreamer {
    session_id: SessionId,
    config: DataStreamerConfig,
    shutdown_tx: broadcast::Sender<()>,
}

impl DataStreamer {
    /// Create a new DataStreamer for a session
    pub fn new(session_id: SessionId, config: DataStreamerConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            session_id,
            config,
            shutdown_tx,
        }
    }

    /// Start the read loop for this streamer
    ///
    /// This spawns an async task that continuously reads from the connection,
    /// batches the data, and emits Tauri events.
    pub async fn start(
        &self,
        app: tauri::AppHandle,
        registry: Arc<RwLock<ConnectionRegistry>>,
    ) {
        let session_id = self.session_id.clone();
        let config = self.config.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut buffer = BytesMut::with_capacity(config.read_buffer_size);
            let mut last_flush = Instant::now();
            let mut bytes_since_flush: usize = 0;

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = Self::read_from_connection(
                        &session_id,
                        &registry,
                        &mut buffer,
                        config.read_buffer_size,
                    ) => {
                        match result {
                            Ok(bytes_read) => {
                                if bytes_read > 0 {
                                    bytes_since_flush += bytes_read;
                                    
                                    if Self::should_flush(
                                        buffer.len(),
                                        bytes_since_flush,
                                        &last_flush,
                                        &config,
                                    ) {
                                        Self::emit_data(
                                            &app,
                                            &session_id,
                                            &mut buffer,
                                        );
                                        last_flush = Instant::now();
                                        bytes_since_flush = 0;
                                    }
                                } else {
                                    if !buffer.is_empty() {
                                        Self::emit_data(
                                            &app,
                                            &session_id,
                                            &mut buffer,
                                        );
                                        last_flush = Instant::now();
                                        bytes_since_flush = 0;
                                    }
                                    tokio::time::sleep(Duration::from_millis(1)).await;
                                }
                            }
                            Err(e) => {
                                if !buffer.is_empty() {
                                    Self::emit_data(
                                        &app,
                                        &session_id,
                                        &mut buffer,
                                    );
                                }
                                tracing::error!(
                                    session_id = %session_id,
                                    error = %e,
                                    "Read error in data streamer"
                                );
                                break;
                            }
                        }
                    }
                }
            }

            tracing::info!(
                session_id = %session_id,
                "Data streamer stopped"
            );
        });
    }

    /// Stop this streamer
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(());
    }

    async fn read_from_connection(
        session_id: &SessionId,
        registry: &Arc<RwLock<ConnectionRegistry>>,
        buffer: &mut BytesMut,
        max_read: usize,
    ) -> Result<usize, String> {
        let connection = {
            let reg = registry.read().await;
            reg.get_by_session(session_id)
        };

        let connection = connection.ok_or_else(|| {
            format!("Connection not found for session: {}", session_id)
        })?;

        let mut read_buf = vec![0u8; max_read];
        let bytes_read = {
            let mut conn = connection.lock().await;
            conn.read(&mut read_buf).await.map_err(|e| {
                format!("Read failed: {}", e)
            })?
        };

        if bytes_read > 0 {
            buffer.extend_from_slice(&read_buf[..bytes_read]);
        }

        Ok(bytes_read)
    }

    fn should_flush(
        buffer_len: usize,
        bytes_since_flush: usize,
        last_flush: &Instant,
        config: &DataStreamerConfig,
    ) -> bool {
        if buffer_len >= config.batch_size {
            return true;
        }

        let elapsed = last_flush.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        if bytes_since_flush < 1024 && elapsed_ms >= config.min_batch_delay_ms {
            return true;
        }

        if elapsed_ms >= config.max_batch_delay_ms {
            return true;
        }

        false
    }

    fn emit_data(
        app: &tauri::AppHandle,
        session_id: &SessionId,
        buffer: &mut BytesMut,
    ) {
        if buffer.is_empty() {
            return;
        }

        let data = buffer.split().freeze();
        let event = DataReceivedEvent {
            session_id: session_id.clone(),
            data: data.to_vec(),
        };

        if let Err(e) = app.emit_all("data_received", event) {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "Failed to emit data_received event"
            );
        }
    }
}

impl std::fmt::Debug for DataStreamer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataStreamer")
            .field("session_id", &self.session_id)
            .field("config", &self.config)
            .finish()
    }
}

/// Manages multiple DataStreamers for different sessions
pub struct DataStreamerManager {
    app: tauri::AppHandle,
    config: DataStreamerConfig,
    streamers: Arc<Mutex<HashMap<SessionId, DataStreamer>>>,
}

impl DataStreamerManager {
    /// Create a new DataStreamerManager
    pub fn new(app: tauri::AppHandle, config: DataStreamerConfig) -> Self {
        Self {
            app,
            config,
            streamers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start streaming data for a session
    ///
    /// This is typically called from the on_connected callback.
    pub async fn start_streaming(
        &self,
        session_id: SessionId,
        registry: Arc<RwLock<ConnectionRegistry>>,
    ) {
        let mut streamers = self.streamers.lock().await;

        if streamers.contains_key(&session_id) {
            tracing::warn!(
                session_id = %session_id,
                "Data streamer already exists for session"
            );
            return;
        }

        let streamer = DataStreamer::new(session_id.clone(), self.config.clone());
        streamer.start(self.app.clone(), registry).await;
        streamers.insert(session_id.clone(), streamer);

        tracing::info!(
            session_id = %session_id,
            "Data streamer started"
        );
    }

    /// Stop streaming data for a session
    ///
    /// This is typically called from the on_disconnected callback.
    pub async fn stop_streaming(&self, session_id: &SessionId) {
        let mut streamers = self.streamers.lock().await;

        if let Some(streamer) = streamers.remove(session_id) {
            streamer.stop();
            tracing::info!(
                session_id = %session_id,
                "Data streamer stopped"
            );
        } else {
            tracing::warn!(
                session_id = %session_id,
                "No data streamer found for session"
            );
        }
    }

    /// Stop all streamers
    pub async fn stop_all(&self) {
        let mut streamers = self.streamers.lock().await;
        for (session_id, streamer) in streamers.drain() {
            streamer.stop();
            tracing::info!(
                session_id = %session_id,
                "Data streamer stopped"
            );
        }
    }

    /// Get the number of active streamers
    pub async fn active_count(&self) -> usize {
        self.streamers.lock().await.len()
    }
}

impl std::fmt::Debug for DataStreamerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataStreamerManager")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_streamer_config_default() {
        let config = DataStreamerConfig::default();
        assert_eq!(config.min_batch_delay_ms, 1);
        assert_eq!(config.max_batch_delay_ms, 16);
        assert_eq!(config.batch_size, 4096);
        assert_eq!(config.read_buffer_size, 16384);
    }

    #[test]
    fn test_should_flush_batch_size() {
        let config = DataStreamerConfig::default();
        let last_flush = Instant::now();

        assert!(DataStreamer::should_flush(
            4096,
            0,
            &last_flush,
            &config
        ));

        assert!(!DataStreamer::should_flush(
            2048,
            0,
            &last_flush,
            &config
        ));
    }

    #[test]
    fn test_should_flush_max_delay() {
        let config = DataStreamerConfig::default();
        let last_flush = Instant::now() - Duration::from_millis(20);

        assert!(DataStreamer::should_flush(
            100,
            0,
            &last_flush,
            &config
        ));
    }

    #[test]
    fn test_should_flush_low_throughput() {
        let config = DataStreamerConfig::default();
        let last_flush = Instant::now() - Duration::from_millis(2);

        assert!(DataStreamer::should_flush(
            100,
            500,
            &last_flush,
            &config
        ));
    }

    #[test]
    fn test_should_not_flush_high_throughput_early() {
        let config = DataStreamerConfig::default();
        let last_flush = Instant::now() - Duration::from_millis(2);

        assert!(!DataStreamer::should_flush(
            100,
            2048,
            &last_flush,
            &config
        ));
    }
}
