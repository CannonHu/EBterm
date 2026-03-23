//! Mock implementations for testing

use async_trait::async_trait;
use std::collections::VecDeque;

use embedded_debugger::connection::types::{
    Connection, ConnectionStats, ConnectionStatus, ConnectionType,
};
use embedded_debugger::connection::ConnectionError;

/// Mock connection for testing with internal buffer simulation
pub struct MockConnection {
    status: ConnectionStatus,
    read_buffer: VecDeque<u8>,
    write_buffer: Vec<u8>,
    stats: ConnectionStats,
}

impl MockConnection {
    /// Create a new MockConnection
    pub fn new() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            read_buffer: VecDeque::new(),
            write_buffer: Vec::new(),
            stats: ConnectionStats::default(),
        }
    }

    /// Create with pre-populated read buffer
    pub fn with_read_data(data: &[u8]) -> Self {
        let mut read_buffer = VecDeque::new();
        read_buffer.extend(data);
        Self {
            status: ConnectionStatus::Disconnected,
            read_buffer,
            write_buffer: Vec::new(),
            stats: ConnectionStats::default(),
        }
    }

    /// Get data written to the mock connection
    pub fn get_written_data(&self) -> Vec<u8> {
        self.write_buffer.clone()
    }

    /// Add data to the read buffer (simulates incoming data)
    pub fn add_read_data(&mut self, data: &[u8]) {
        self.read_buffer.extend(data);
    }

    /// Clear all buffers
    pub fn clear_buffers(&mut self) {
        self.read_buffer.clear();
        self.write_buffer.clear();
    }
}

impl Default for MockConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Connection for MockConnection {
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Serial
    }

    fn status(&self) -> ConnectionStatus {
        self.status
    }

    async fn connect(&mut self) -> Result<(), ConnectionError> {
        self.status = ConnectionStatus::Connected;
        self.stats.connected_at = Some(std::time::Instant::now());
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        self.status = ConnectionStatus::Disconnected;
        self.stats.connected_at = None;
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError> {
        if !self.is_connected() {
            return Err(ConnectionError::NotConnected);
        }

        let mut count = 0;
        for byte in buf.iter_mut() {
            match self.read_buffer.pop_front() {
                Some(b) => {
                    *byte = b;
                    count += 1;
                }
                None => break,
            }
        }

        if count > 0 {
            self.stats.bytes_received += count as u64;
            self.stats.packets_received += 1;
        }

        Ok(count)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        if !self.is_connected() {
            return Err(ConnectionError::NotConnected);
        }

        self.write_buffer.extend_from_slice(buf);
        let len = buf.len();

        self.stats.bytes_sent += len as u64;
        self.stats.packets_sent += 1;

        Ok(len)
    }

    async fn flush(&mut self) -> Result<(), ConnectionError> {
        // Clear write buffer on flush
        self.write_buffer.clear();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        matches!(self.status, ConnectionStatus::Connected)
    }

    fn stats(&self) -> ConnectionStats {
        self.stats.clone()
    }

    fn clear_stats(&mut self) {
        let connected_at = self.stats.connected_at;
        self.stats = ConnectionStats {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            connected_at,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_connection_default_state() {
        let conn = MockConnection::new();
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_connect() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        assert_eq!(conn.status(), ConnectionStatus::Connected);
        assert!(conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_disconnect() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.disconnect().await.unwrap();
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());
    }

    #[tokio::test]
    async fn test_mock_connection_write() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        let data = b"Hello, World!";
        let written = conn.write(data).await.unwrap();
        assert_eq!(written, data.len());

        let written_data = conn.get_written_data();
        assert_eq!(written_data, data);
    }

    #[tokio::test]
    async fn test_mock_connection_read() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.add_read_data(b"test data");

        let mut buf = [0u8; 1024];
        let read = conn.read(&mut buf).await.unwrap();
        assert_eq!(read, 9);
        assert_eq!(&buf[..read], b"test data");
    }

    #[tokio::test]
    async fn test_mock_connection_read_empty_buffer() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        let mut buf = [0u8; 1024];
        let read = conn.read(&mut buf).await.unwrap();
        assert_eq!(read, 0);
    }

    #[tokio::test]
    async fn test_mock_connection_flush() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();

        conn.write(b"data").await.unwrap();
        assert!(!conn.get_written_data().is_empty());

        conn.flush().await.unwrap();
        assert!(conn.get_written_data().is_empty());
    }

    #[tokio::test]
    async fn test_mock_connection_stats() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.write(b"test").await.unwrap();

        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, 4);
        assert_eq!(stats.packets_sent, 1);
    }

    #[tokio::test]
    async fn test_mock_connection_clear_stats() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.write(b"test").await.unwrap();

        conn.clear_stats();
        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.packets_sent, 0);
    }

    #[tokio::test]
    async fn test_mock_connection_not_connected_write() {
        let mut conn = MockConnection::new();
        let result = conn.write(b"test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_connection_not_connected_read() {
        let mut conn = MockConnection::new();
        let mut buf = [0u8; 10];
        let result = conn.read(&mut buf).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_connection_with_read_data() {
        let conn = MockConnection::with_read_data(b"initial data");
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_mock_connection_clear_buffers() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.add_read_data(b"read");
        conn.write(b"write").await.unwrap();

        conn.clear_buffers();
        assert!(conn.get_written_data().is_empty());

        let mut buf = [0u8; 10];
        let read = conn.read(&mut buf).await.unwrap();
        assert_eq!(read, 0);
    }

    #[tokio::test]
    async fn test_mock_connection_stats_received() {
        let mut conn = MockConnection::new();
        conn.connect().await.unwrap();
        conn.add_read_data(b"received");

        let mut buf = [0u8; 1024];
        conn.read(&mut buf).await.unwrap();

        let stats = conn.stats();
        assert_eq!(stats.bytes_received, 8);
        assert_eq!(stats.packets_received, 1);
    }
}