//! Connection registry for managing connection instances
//!
//! Uses a Vec-based index system for O(1) lookups with better cache locality
//! than HashMap-based approaches.

use std::collections::HashMap;

use crate::connection::types::ConnectionHandle;
use crate::session::SessionError;
use crate::session::SessionId;

/// Registry for managing connection instances using Vec indices
///
/// Uses a Vec to store connections with SessionId -> index mapping.
/// Index (usize) serves as the connection identifier.
pub struct ConnectionRegistry {
    /// Vec of connections, index is the connection ID
    /// None represents an empty/recycled slot
    connections: Vec<Option<ConnectionHandle>>,
    /// SessionId -> Vec index mapping
    session_to_index: HashMap<SessionId, usize>,
    /// Recycled indices for reuse (LIFO for cache locality)
    free_indices: Vec<usize>,
}

impl ConnectionRegistry {
    /// Create a new empty ConnectionRegistry
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            session_to_index: HashMap::new(),
            free_indices: Vec::new(),
        }
    }

    /// Create with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            connections: Vec::with_capacity(capacity),
            session_to_index: HashMap::with_capacity(capacity),
            free_indices: Vec::new(),
        }
    }

    /// Insert a new connection and return its index
    ///
    /// Reuses free indices if available, otherwise pushes to Vec
    pub fn insert(
        &mut self,
        session_id: SessionId,
        connection: ConnectionHandle,
    ) -> Result<usize, SessionError> {
        if self.session_to_index.contains_key(&session_id) {
            return Err(SessionError::AlreadyExists { id: session_id });
        }

        // Get index: reuse free slot or create new
        let index = if let Some(free_index) = self.free_indices.pop() {
            // Reuse existing slot
            self.connections[free_index] = Some(connection);
            free_index
        } else {
            // Create new slot
            let new_index = self.connections.len();
            self.connections.push(Some(connection));
            new_index
        };

        self.session_to_index.insert(session_id, index);
        Ok(index)
    }

    /// Get connection by index
    pub fn get_by_index(&self, index: usize) -> Option<ConnectionHandle> {
        self.connections.get(index)?.as_ref().cloned()
    }

    /// Get connection by SessionId
    pub fn get_by_session(&self, session_id: &SessionId) -> Option<ConnectionHandle> {
        let index = *self.session_to_index.get(session_id)?;
        self.get_by_index(index)
    }

    /// Get index by SessionId
    pub fn get_index(&self, session_id: &SessionId) -> Option<usize> {
        self.session_to_index.get(session_id).copied()
    }

    /// Remove connection by SessionId
    /// Returns the connection and its index
    pub fn remove(&mut self, session_id: &SessionId) -> Option<(usize, ConnectionHandle)> {
        let index = self.session_to_index.remove(session_id)?;

        // Take connection from slot
        let connection = std::mem::replace(&mut self.connections[index], None)?;

        // Recycle index
        self.free_indices.push(index);

        Some((index, connection))
    }

    /// Remove connection by index
    pub fn remove_by_index(&mut self, index: usize) -> Option<(SessionId, ConnectionHandle)> {
        let session_id = self
            .session_to_index
            .iter()
            .find(|(_, idx)| **idx == index)
            .map(|(sid, _)| sid.clone())?;

        let connection = std::mem::replace(&mut self.connections[index], None)?;

        self.session_to_index.remove(&session_id);

        self.free_indices.push(index);

        Some((session_id, connection))
    }

    /// Check if session exists
    pub fn contains_session(&self, session_id: &SessionId) -> bool {
        self.session_to_index.contains_key(session_id)
    }

    /// Check if index is valid and occupied
    pub fn contains_index(&self, index: usize) -> bool {
        matches!(self.connections.get(index), Some(Some(_)))
    }

    /// Get number of active connections
    pub fn len(&self) -> usize {
        self.session_to_index.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.session_to_index.is_empty()
    }

    /// Get capacity (number of slots, including empty ones)
    pub fn capacity(&self) -> usize {
        self.connections.len()
    }

    /// Get number of available recycled indices
    pub fn free_count(&self) -> usize {
        self.free_indices.len()
    }

    /// Iterate over all active connections
    pub fn iter(&self) -> impl Iterator<Item = (&SessionId, usize, &ConnectionHandle)> {
        self.session_to_index.iter().filter_map(|(sid, idx)| {
            self.connections[*idx]
                .as_ref()
                .map(|conn| (sid, *idx, conn))
        })
    }

    /// Clear all connections
    pub fn clear(&mut self) {
        self.connections.clear();
        self.session_to_index.clear();
        self.free_indices.clear();
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ConnectionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionRegistry")
            .field("active_count", &self.len())
            .field("total_slots", &self.capacity())
            .field("free_slots", &self.free_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{ConnectionError, types::{
        ConnectionStatus, ConnectionStats, ConnectionType,
    }};
    use async_trait::async_trait;

    fn create_mock_connection() -> ConnectionHandle {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        Arc::new(Mutex::new(Box::new(MockConnection::new()) as Box<dyn crate::connection::types::Connection>))
    }

    struct MockConnection {
        status: ConnectionStatus,
        stats: ConnectionStats,
    }

    impl MockConnection {
        fn new() -> Self {
            Self {
                status: ConnectionStatus::Disconnected,
                stats: ConnectionStats::default(),
            }
        }
    }

    #[async_trait]
    impl crate::connection::types::Connection for MockConnection {
        fn connection_type(&self) -> ConnectionType {
            ConnectionType::Serial
        }

        fn status(&self) -> ConnectionStatus {
            self.status
        }

        async fn connect(&mut self) -> Result<(), ConnectionError> {
            self.status = ConnectionStatus::Connected;
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<(), ConnectionError> {
            self.status = ConnectionStatus::Disconnected;
            Ok(())
        }

        async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ConnectionError> {
            Ok(0)
        }

        async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
            self.stats.bytes_sent += buf.len() as u64;
            self.stats.packets_sent += 1;
            Ok(buf.len())
        }

        async fn flush(&mut self) -> Result<(), ConnectionError> {
            Ok(())
        }

        fn is_connected(&self) -> bool {
            matches!(self.status, ConnectionStatus::Connected)
        }

        fn stats(&self) -> ConnectionStats {
            self.stats.clone()
        }

        fn clear_stats(&mut self) {
            self.stats = ConnectionStats::default();
        }
    }

    #[test]
    fn test_new_registry_is_empty() {
        let registry = ConnectionRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.capacity(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut registry = ConnectionRegistry::new();
        let conn = create_mock_connection();
        let session_id = "session-1".to_string();

        let index = registry.insert(session_id.clone(), conn.clone()).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get_by_index(index).is_some());
        assert!(registry.get_by_session(&session_id).is_some());
        assert_eq!(registry.get_index(&session_id), Some(index));
    }

    #[test]
    fn test_duplicate_insert_fails() {
        let mut registry = ConnectionRegistry::new();
        let conn1 = create_mock_connection();
        let conn2 = create_mock_connection();

        registry.insert("session-1".to_string(), conn1).unwrap();

        let result = registry.insert("session-1".to_string(), conn2);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove() {
        let mut registry = ConnectionRegistry::new();
        let conn = create_mock_connection();
        let session_id = "session-1".to_string();

        let index = registry.insert(session_id.clone(), conn).unwrap();
        assert_eq!(registry.len(), 1);

        let (removed_index, _removed_conn) = registry.remove(&session_id).unwrap();
        assert_eq!(removed_index, index);
        assert_eq!(registry.len(), 0);
        assert!(registry.get_by_index(index).is_none());
        assert!(registry.get_by_session(&session_id).is_none());
    }

    #[test]
    fn test_index_reuse() {
        let mut registry = ConnectionRegistry::new();

        let conn1 = create_mock_connection();
        let conn2 = create_mock_connection();
        let session1 = "session-1".to_string();
        let session2 = "session-2".to_string();

        let idx1 = registry.insert(session1.clone(), conn1).unwrap();
        registry.remove(&session1);

        let idx2 = registry.insert(session2, conn2).unwrap();

        // Should reuse the freed index
        assert_eq!(idx1, idx2);
        assert_eq!(registry.free_count(), 0);
    }

    #[test]
    fn test_multiple_indices() {
        let mut registry = ConnectionRegistry::new();

        for i in 0..10 {
            let conn = create_mock_connection();
            let idx = registry.insert(format!("session-{}", i), conn).unwrap();
            assert_eq!(idx, i); // Should be sequential
        }

        assert_eq!(registry.len(), 10);
        assert_eq!(registry.capacity(), 10);
    }

    #[test]
    fn test_get_nonexistent() {
        let registry = ConnectionRegistry::new();
        let nonexistent = "nonexistent".to_string();

        assert!(registry.get_by_session(&nonexistent).is_none());
        assert!(registry.get_by_index(0).is_none());
        assert!(registry.get_index(&nonexistent).is_none());
    }

    #[test]
    fn test_contains() {
        let mut registry = ConnectionRegistry::new();
        let conn = create_mock_connection();
        let session_id = "session-1".to_string();

        assert!(!registry.contains_session(&session_id));

        let idx = registry.insert(session_id.clone(), conn).unwrap();

        assert!(registry.contains_session(&session_id));
        assert!(registry.contains_index(idx));
        assert!(!registry.contains_index(idx + 1));
    }

    #[test]
    fn test_iter() {
        let mut registry = ConnectionRegistry::new();

        for i in 0..3 {
            let conn = create_mock_connection();
            registry.insert(format!("session-{}", i), conn).unwrap();
        }

        let count = registry.iter().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_clear() {
        let mut registry = ConnectionRegistry::new();

        for i in 0..5 {
            let conn = create_mock_connection();
            registry.insert(format!("session-{}", i), conn).unwrap();
        }

        assert_eq!(registry.len(), 5);

        registry.clear();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.capacity(), 0);
    }

    #[test]
    fn test_debug_format() {
        let mut registry = ConnectionRegistry::new();
        let conn = create_mock_connection();

        registry.insert("session-1".to_string(), conn).unwrap();

        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("ConnectionRegistry"));
        assert!(debug_str.contains("active_count"));
        assert!(debug_str.contains("1"));
    }

    #[test]
    fn test_capacity_and_free_count() {
        let mut registry = ConnectionRegistry::with_capacity(10);

        assert_eq!(registry.capacity(), 0); // No slots used yet

        for i in 0..5 {
            let conn = create_mock_connection();
            registry.insert(format!("session-{}", i), conn).unwrap();
        }

        assert_eq!(registry.capacity(), 5);
        assert_eq!(registry.free_count(), 0);

        // Remove some
        registry.remove(&"session-0".to_string());
        registry.remove(&"session-1".to_string());

        assert_eq!(registry.free_count(), 2);
        assert_eq!(registry.capacity(), 5); // Still 5 slots
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut registry = ConnectionRegistry::new();
        let nonexistent = "nonexistent".to_string();

        assert!(registry.remove(&nonexistent).is_none());
    }

    #[test]
    fn test_remove_by_index() {
        let mut registry = ConnectionRegistry::new();
        let conn = create_mock_connection();
        let session_id = "session-1".to_string();

        let idx = registry.insert(session_id.clone(), conn).unwrap();

        let (removed_session_id, _) = registry.remove_by_index(idx).unwrap();
        assert_eq!(removed_session_id, session_id);

        assert!(registry.get_by_index(idx).is_none());
    }

    #[test]
    fn test_default() {
        let registry: ConnectionRegistry = Default::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_insert_after_clear() {
        let mut registry = ConnectionRegistry::new();

        // Insert and clear multiple times
        for round in 0..3 {
            for i in 0..5 {
                let conn = create_mock_connection();
                registry
                    .insert(format!("round{}-session{}", round, i), conn)
                    .unwrap();
            }
            assert_eq!(registry.len(), 5);
            registry.clear();
            assert!(registry.is_empty());
        }
    }

    #[test]
    fn test_concurrent_insert_and_remove() {
        // Simulate rapid insert/remove cycles
        let mut registry = ConnectionRegistry::new();

        for i in 0..100 {
            let conn = create_mock_connection();
            let idx = registry.insert(format!("session-{}", i), conn).unwrap();

            // Remove every 3rd connection
            if i % 3 == 0 && i > 0 {
                let remove_idx = i - 3;
                registry.remove(&format!("session-{}", remove_idx));
            }
        }

        // Verify state is consistent
        assert!(registry.len() > 0);
        assert!(registry.capacity() <= 100);
    }
}
