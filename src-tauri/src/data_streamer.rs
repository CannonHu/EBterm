//! Simplified data streaming module for Tauri integration
//!
//! Manages background read loops for connected connections using time-window batching.
//! This implementation uses tokio::time::timeout to enforce a 16ms batching window,
//! ensuring data is sent in batches rather than byte-by-byte.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tauri::Manager;

use embedded_debugger::connection::ConnectionHandle;

use crate::ipc::DataReceivedEvent;

const BATCH_INTERVAL_MS: u64 = 16;
const MAX_BATCH_SIZE: usize = 16384;
const READ_BUFFER_SIZE: usize = 4096;

pub struct DataStreamerManager {
    app: tauri::AppHandle,
    streamers: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl DataStreamerManager {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            streamers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_streaming(
        &self,
        connection_id: String,
        connection: ConnectionHandle,
    ) {
        let mut streamers = self.streamers.lock().await;

        if streamers.contains_key(&connection_id) {
            tracing::warn!(
                connection_id = %connection_id,
                "Data streamer already exists for connection"
            );
            return;
        }

        let app = self.app.clone();
        let connection_id_clone = connection_id.clone();

        let handle = tokio::spawn(async move {
            start_batch_streamer(connection_id_clone, connection, app).await;
        });

        streamers.insert(connection_id.clone(), handle);

        tracing::info!(
            connection_id = %connection_id,
            "Data streamer started"
        );
    }

    pub async fn stop_streaming(&self, connection_id: &str) {
        let mut streamers = self.streamers.lock().await;

        if let Some(handle) = streamers.remove(connection_id) {
            handle.abort();
            tracing::info!(
                connection_id = %connection_id,
                "Data streamer stopped"
            );
        } else {
            tracing::warn!(
                connection_id = %connection_id,
                "No data streamer found for connection"
            );
        }
    }

    pub async fn stop_all(&self) {
        let mut streamers = self.streamers.lock().await;
        for (connection_id, handle) in streamers.drain() {
            handle.abort();
            tracing::info!(
                connection_id = %connection_id,
                "Data streamer stopped"
            );
        }
    }

    pub async fn active_count(&self) -> usize {
        self.streamers.lock().await.len()
    }
}

impl std::fmt::Debug for DataStreamerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataStreamerManager").finish()
    }
}

async fn start_batch_streamer(
    connection_id: String,
    connection: ConnectionHandle,
    app: tauri::AppHandle,
) {
    let mut batch_buffer = Vec::with_capacity(MAX_BATCH_SIZE);
    let mut last_batch_time = Instant::now();
    let mut read_buf = [0u8; READ_BUFFER_SIZE];

    loop {
        let timeout_duration = Duration::from_millis(BATCH_INTERVAL_MS)
            .saturating_sub(last_batch_time.elapsed());

        let read_result = timeout(
            timeout_duration,
            async {
                let mut conn = connection.lock().await;
                conn.read(&mut read_buf).await
            },
        ).await;

        match read_result {
            Ok(Ok(0)) => {
                if !batch_buffer.is_empty() {
                    send_batch(&app, &connection_id, &batch_buffer).await;
                }
                break;
            }
            Ok(Ok(n)) => {
                batch_buffer.extend_from_slice(&read_buf[..n]);

                if batch_buffer.len() >= MAX_BATCH_SIZE {
                    send_batch(&app, &connection_id, &batch_buffer).await;
                    batch_buffer.clear();
                    last_batch_time = Instant::now();
                }
            }
            Ok(Err(_)) => {
                if !batch_buffer.is_empty() {
                    send_batch(&app, &connection_id, &batch_buffer).await;
                }
                break;
            }
            Err(_) => {
                if !batch_buffer.is_empty() {
                    send_batch(&app, &connection_id, &batch_buffer).await;
                    batch_buffer.clear();
                }
                last_batch_time = Instant::now();
            }
        }
    }

    tracing::info!(
        connection_id = %connection_id,
        "Data streamer stopped"
    );
}

async fn send_batch(app: &tauri::AppHandle, connection_id: &str, data: &[u8]) {
    let event = DataReceivedEvent {
        connection_id: connection_id.to_string(),
        data: data.to_vec(),
    };

    let _ = app.emit_all("data_received", event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_constants() {
        assert_eq!(BATCH_INTERVAL_MS, 16);
        assert_eq!(MAX_BATCH_SIZE, 16384);
        assert_eq!(READ_BUFFER_SIZE, 4096);
    }
}
