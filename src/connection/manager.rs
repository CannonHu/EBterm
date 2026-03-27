use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::types::{ConnectionConfig, ConnectionStats, ConnectionType};
use super::{Connection, ConnectionError};

#[derive(Clone)]
struct MockConnection {
    connected: bool,
    stats: ConnectionStats,
}

#[async_trait::async_trait]
impl Connection for MockConnection {
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Telnet
    }

    fn status(&self) -> super::ConnectionStatus {
        if self.connected {
            super::ConnectionStatus::Connected
        } else {
            super::ConnectionStatus::Disconnected
        }
    }

    async fn connect(&mut self) -> Result<(), ConnectionError> {
        self.connected = true;
        self.stats.connected_at = Some(std::time::Instant::now());
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        self.connected = false;
        self.stats.connected_at = None;
        Ok(())
    }

    async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ConnectionError> {
        Ok(0)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        let len = buf.len();
        
        self
        .stats
        .bytes_sent += len as u64;
        self.stats.packets_sent += 1;
        Ok(len)
    }

    async fn flush(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn stats(&self) -> ConnectionStats {
        self.stats.clone()
    }

    fn clear_stats(&mut self) {
        self.stats = ConnectionStats::default();
    }
}

#[derive(Clone)]
struct ConnectionEntry {
    id: String,
    name: String,
    connection: Arc<Mutex<Box<dyn Connection>>>,
    created_at: std::time::Instant,
    stats: ConnectionStats,
}

pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, ConnectionEntry>>>,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub stats: ConnectionStats,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionManagerStats {
    pub total_created: u64,
    pub total_closed: u64,
    pub active_connections: usize,
    pub peak_concurrent: usize,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_connection(
        &self,
        name: String,
        _config: ConnectionConfig,
    ) -> Result<String, ConnectionError> {
        let connection_id = uuid::Uuid::new_v4().to_string();

        let connection: Arc<Mutex<Box<dyn Connection>>> = Arc::new(Mutex::new(
            Box::new(MockConnection {
                connected: false,
                stats: ConnectionStats::default(),
            }),
        ));

        let entry = ConnectionEntry {
            id: connection_id.clone(),
            name,
            connection,
            created_at: std::time::Instant::now(),
            stats: ConnectionStats::default(),
        };

        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), entry);

        Ok(connection_id)
    }

    pub async fn connect(
        &self,
        connection_id: &str,
        _config: ConnectionConfig,
    ) -> Result<(), ConnectionError> {
        let mut connections = self.connections.write().await;

        let entry = connections
            .get_mut(connection_id)
            .ok_or_else(|| ConnectionError::NotFound {
                port: connection_id.to_string(),
            })?;

        let mut conn = entry.connection.lock().await;
        conn.connect().await?;

        entry.stats.connected_at = Some(std::time::Instant::now());

        Ok(())
    }

    pub async fn disconnect(&self, connection_id: &str) -> Result<(), ConnectionError> {
        let connections = self.connections.read().await;

        let entry = connections
            .get(connection_id)
            .ok_or_else(|| ConnectionError::NotFound {
                port: connection_id.to_string(),
            })?;

        let mut conn = entry.connection.lock().await;
        conn.disconnect().await?;

        Ok(())
    }

    pub async fn write(
        &self,
        connection_id: &str,
        data: Vec<u8>,
    ) -> Result<usize, ConnectionError> {
        let connections = self.connections.read().await;

        let entry = connections
            .get(connection_id)
            .ok_or_else(|| ConnectionError::NotFound {
                port: connection_id.to_string(),
            })?;

        let mut conn = entry.connection.lock().await;

        if !conn.is_connected() {
            return Err(ConnectionError::NotConnected);
        }

        let bytes_written = conn.write(&data).await?;

        drop(conn);
        let mut connections = self.connections.write().await;
        if let Some(entry) = connections.get_mut(connection_id) {
            entry.stats.bytes_sent += bytes_written as u64;
            entry.stats.packets_sent += 1;
        }

        Ok(bytes_written)
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectionInfo> {
        let connections = self.connections.read().await;

        let entry = connections.get(connection_id)?;

        let status = {
            let conn = entry.connection.lock().await;
            conn.status()
        };

        Some(ConnectionInfo {
            id: entry.id.clone(),
            name: entry.name.clone(),
            status: status.to_string(),
            created_at: entry.created_at.elapsed().as_secs() as i64,
            stats: entry.stats.clone(),
        })
    }

    pub async fn list_connections(&self) -> Vec<ConnectionInfo> {
        let connections = self.connections.read().await;

        let mut result = Vec::new();

        for entry in connections.values() {
            let status = {
                let conn = entry.connection.lock().await;
                conn.status()
            };

            result.push(ConnectionInfo {
                id: entry.id.clone(),
                name: entry.name.clone(),
                status: status.to_string(),
                created_at: entry.created_at.elapsed().as_secs() as i64,
                stats: entry.stats.clone(),
            });
        }

        result
    }

    pub async fn stats(&self) -> ConnectionManagerStats {
        let connections = self.connections.read().await;
        let count = connections.len();

        ConnectionManagerStats {
            total_created: 0,
            total_closed: 0,
            active_connections: count,
            peak_concurrent: count,
        }
    }

    pub async fn close_connection(&self, connection_id: &str) -> Result<(), ConnectionError> {
        let mut connections = self.connections.write().await;

        let entry = connections
            .remove(connection_id)
            .ok_or_else(|| ConnectionError::NotFound {
                port: connection_id.to_string(),
            })?;

        let mut conn = entry.connection.lock().await;
        if conn.is_connected() {
            conn.disconnect().await?;
        }

        Ok(())
    }

    pub async fn get_connection_handle(&self, connection_id: &str) -> Option<Arc<Mutex<Box<dyn super::Connection>>>> {
        let connections = self.connections.read().await;
        connections.get(connection_id).map(|entry| entry.connection.clone())
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::new();
        let connections = futures::executor::block_on(manager.list_connections());
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_connection_creation() {
        let manager = ConnectionManager::new();

        let connection_id = manager
            .create_connection("Test Connection".to_string(), super::super::ConnectionConfig::Serial(super::super::SerialConfig::default()))
            .await
            .expect("Failed to create connection");

        let connection_info = manager
            .get_connection(&connection_id)
            .await
            .expect("Connection not found");

        assert_eq!(connection_info.name, "Test Connection");
        assert_eq!(connection_info.id, connection_id);
    }

    #[tokio::test]
    async fn test_list_connections() {
        let manager = ConnectionManager::new();

        let connections = manager.list_connections().await;
        assert!(connections.is_empty());

        let manager = ConnectionManager::new();

        let _id1 = manager
            .create_connection(
                "Connection 1".to_string(),
                super::super::ConnectionConfig::Telnet(super::super::TelnetConfig::default()),
            )
            .await
            .unwrap();
        let _id2 = manager
            .create_connection(
                "Connection 2".to_string(),
                super::super::ConnectionConfig::Telnet(super::super::TelnetConfig::default()),
            )
            .await
            .unwrap();

        let connections = manager.list_connections().await;
        assert_eq!(connections.len(), 2);
    }

    #[tokio::test]
    async fn test_close_nonexistent_connection() {
        let manager = ConnectionManager::new();

        let result = manager.close_connection("nonexistent-id").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_get_nonexistent_connection() {
        let manager = ConnectionManager::new();

        let connection = manager.get_connection("nonexistent-id").await;
        assert!(connection.is_none());
    }

    #[tokio::test]
    async fn test_write_nonexistent_connection() {
        let manager = ConnectionManager::new();

        let result = manager.write("nonexistent-id", vec![1, 2, 3]).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_connection() {
        let manager = ConnectionManager::new();

        let result = manager.disconnect("nonexistent-id").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConnectionError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_manager_stats() {
        let manager = ConnectionManager::new();

        let stats = manager.stats().await;
        assert_eq!(stats.active_connections, 0);

        let _id = manager
            .create_connection("Test".to_string(), super::super::ConnectionConfig::Telnet(super::super::TelnetConfig::default()))
            .await
            .unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.active_connections, 1);
    }

    #[tokio::test]
    async fn test_close_connection() {
        let manager = ConnectionManager::new();

        let connection_id = manager
            .create_connection("Test".to_string(), super::super::ConnectionConfig::Telnet(super::super::TelnetConfig::default()))
            .await
            .unwrap();

        assert!(manager.get_connection(&connection_id).await.is_some());

        manager.close_connection(&connection_id).await.unwrap();

        assert!(manager.get_connection(&connection_id).await.is_none());
    }
}
