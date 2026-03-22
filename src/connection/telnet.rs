//! Telnet connection implementation
//!
//! Provides Telnet connectivity using raw TCP (tokio::net::TcpStream).
//! For embedded debugging, devices typically use UART-to-Telnet adapters
//! that relay raw data without Telnet protocol negotiation.

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{Connection, ConnectionError, ConnectionStatus, ConnectionStats, ConnectionType};
use super::types::TelnetConfig;

/// Telnet connection implementation
#[derive(Debug)]
pub struct TelnetConnection {
    config: TelnetConfig,
    status: ConnectionStatus,
    stats: ConnectionStats,
    stream: Option<TcpStream>,
}

impl TelnetConnection {
    pub fn new(config: TelnetConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Disconnected,
            stats: ConnectionStats::default(),
            stream: None,
        }
    }

    pub fn config(&self) -> &TelnetConfig {
        &self.config
    }

    fn addr_string(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }
}

#[async_trait]
impl Connection for TelnetConnection {
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Telnet
    }

    fn status(&self) -> ConnectionStatus {
        self.status
    }

    async fn connect(&mut self) -> Result<(), ConnectionError> {
        if self.status == ConnectionStatus::Connected {
            return Ok(());
        }

        self.status = ConnectionStatus::Connecting;

        let addr = self.addr_string();
        let timeout_dur = std::time::Duration::from_secs(self.config.connect_timeout_secs);

        let stream = tokio::time::timeout(timeout_dur, TcpStream::connect(&addr))
            .await
            .map_err(|_| ConnectionError::Timeout {
                port: addr.clone(),
                timeout_ms: self.config.connect_timeout_secs * 1000,
            })?
            .map_err(ConnectionError::Io)?;

        self.stream = Some(stream);
        self.status = ConnectionStatus::Connected;
        self.stats.connected_at = Some(std::time::Instant::now());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        self.status = ConnectionStatus::Disconnected;
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError> {
        let stream = self.stream.as_mut()
            .ok_or(ConnectionError::NotConnected)?;

        match stream.read(buf).await {
            Ok(0) => Err(ConnectionError::NotConnected),
            Ok(n) => {
                self.stats.bytes_received += n as u64;
                self.stats.packets_received += 1;
                Ok(n)
            }
            Err(e) => Err(ConnectionError::Io(e)),
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        let stream = self.stream.as_mut()
            .ok_or(ConnectionError::NotConnected)?;

        stream.write_all(buf).await
            .map_err(ConnectionError::Io)?;

        self.stats.bytes_sent += buf.len() as u64;
        self.stats.packets_sent += 1;

        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), ConnectionError> {
        if let Some(stream) = self.stream.as_mut() {
            stream.flush().await
                .map_err(ConnectionError::Io)?;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.status == ConnectionStatus::Connected && self.stream.is_some()
    }

    fn stats(&self) -> ConnectionStats {
        self.stats.clone()
    }

    fn clear_stats(&mut self) {
        self.stats = ConnectionStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telnet_connection_creation() {
        let config = TelnetConfig {
            host: "192.168.1.1".to_string(),
            port: 23,
            connect_timeout_secs: 10,
        };

        let conn = TelnetConnection::new(config);

        assert_eq!(conn.connection_type(), ConnectionType::Telnet);
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());
        assert_eq!(conn.config().host, "192.168.1.1");
        assert_eq!(conn.config().port, 23);
    }

    #[test]
    fn test_telnet_connection_addr_string() {
        let config = TelnetConfig {
            host: "192.168.1.1".to_string(),
            port: 23,
            connect_timeout_secs: 10,
        };

        let conn = TelnetConnection::new(config);
        assert_eq!(conn.addr_string(), "192.168.1.1:23");
    }

    #[test]
    fn test_telnet_connection_stats() {
        let config = TelnetConfig::default();
        let mut conn = TelnetConnection::new(config);

        let stats = conn.stats();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
        assert!(stats.connected_at.is_none());

        conn.clear_stats();
        let stats_after_clear = conn.stats();
        assert_eq!(stats_after_clear.bytes_sent, 0);
    }
}
