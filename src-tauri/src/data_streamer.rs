//! Data streaming functions for connection context
//!
//! Provides background read loops for connected connections using time-window batching.

use std::time::{Duration, Instant};
use tokio::time::timeout;
use tauri::{Emitter, Manager};

use ebterm::connection::ConnectionHandle;

use crate::ipc::DataReceivedEvent;
use crate::state::AppState;

const BATCH_INTERVAL_MS: u64 = 16;
const MAX_BATCH_SIZE: usize = 16384;
const READ_BUFFER_SIZE: usize = 4096;

/// Start a background batch streamer for a connection
///
/// This function runs in a tokio task and reads data from connection
/// in batches, emitting events to frontend every ~16ms.
pub async fn start_batch_streamer(
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
            Ok(Err(e)) => {
                tracing::error!("[data_streamer] read error: {}", e);
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
}

async fn send_batch(app: &tauri::AppHandle, connection_id: &str, data: &[u8]) {
    // Log of output data from device
    let state = app.state::<AppState>();
    let mut connections = state.connections.write().await;
    if let Some(ctx) = connections.get_mut(connection_id) {
        if let Some(logger) = &mut ctx.logger {
            // Convert bytes to text, clean ANSI/control chars, and log
            let text = String::from_utf8_lossy(data);
            let _ = logger.write(&text);
        }
    }
    drop(connections);

    let event = DataReceivedEvent {
        session_id: connection_id.to_string(),
        data: data.to_vec(),
    };
    let _ = app.emit("data_received", event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch() {
        assert_eq!(BATCH_INTERVAL_MS, 16);
        assert_eq!(MAX_BATCH_SIZE, 16384);
        assert_eq!(READ_BUFFER_SIZE, 4096);
    }
}
