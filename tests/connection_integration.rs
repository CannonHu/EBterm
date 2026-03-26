//! Connection manager integration tests
//!
//! Tests full connection lifecycle, concurrency, and state management

use embedded_debugger::connection::{ConnectionManager, ConnectionConfig, SerialConfig, TelnetConfig};
use embedded_debugger::connection::types::{DataBits, Parity, StopBits, FlowControl};

#[tokio::test]
async fn test_connection_full_lifecycle() {
    let manager = ConnectionManager::new();

    let connection_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/null".to_string(),
        baud_rate: 115200,
        data_bits: DataBits::Eight,
        parity: Parity::None,
        stop_bits: StopBits::One,
        flow_control: FlowControl::None,
    });

    let connection_id = manager
        .create_connection("test-connection".to_string(), connection_config)
        .await
        .expect("Failed to create connection");

    let connection_info = manager
        .get_connection(&connection_id)
        .await
        .expect("Connection not found");

    assert_eq!(connection_info.name, "test-connection");
    assert_eq!(connection_info.id, connection_id);

    manager.close_connection(&connection_id).await.expect("Failed to close");

    assert!(manager.get_connection(&connection_id).await.is_none());
}

#[tokio::test]
async fn test_concurrent_connections_isolation() {
    let manager = ConnectionManager::new();
    let mut connection_ids = Vec::new();

    for i in 0..5 {
        let connection_config = ConnectionConfig::Serial(SerialConfig {
            port: format!("/dev/ttyUSB{}", i),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        });

        let id = manager
            .create_connection(format!("connection-{}", i), connection_config)
            .await
            .expect("Failed to create connection");
        connection_ids.push(id);
    }

    for (i, id) in connection_ids.iter().enumerate() {
        let connection_info = manager.get_connection(id).await.expect("Connection not found");
        assert_eq!(connection_info.name, format!("connection-{}", i));
    }

    for id in &connection_ids {
        manager.close_connection(id).await.expect("Failed to close");
    }

    for id in &connection_ids {
        assert!(manager.get_connection(id).await.is_none());
    }
}

#[tokio::test]
async fn test_list_connections() {
    let manager = ConnectionManager::new();

    let connections = manager.list_connections().await;
    assert!(connections.is_empty());

    let connection_config = ConnectionConfig::Telnet(TelnetConfig {
        host: "localhost".to_string(),
        port: 23,
        connect_timeout_secs: 30,
    });

    let _id1 = manager
        .create_connection("telnet-1".to_string(), connection_config.clone())
        .await
        .unwrap();
    let _id2 = manager
        .create_connection("telnet-2".to_string(), connection_config)
        .await
        .unwrap();

    let connections = manager.list_connections().await;
    assert_eq!(connections.len(), 2);
}

#[tokio::test]
async fn test_write_nonexistent_connection() {
    let manager = ConnectionManager::new();

    let result = manager.write("nonexistent-id", vec![1, 2, 3]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_disconnect_nonexistent_connection() {
    let manager = ConnectionManager::new();

    let result = manager.disconnect("nonexistent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_close_nonexistent_connection() {
    let manager = ConnectionManager::new();

    let result = manager.close_connection("nonexistent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_manager_stats() {
    let manager = ConnectionManager::new();

    let stats = manager.stats().await;
    assert_eq!(stats.active_connections, 0);

    let connection_config = ConnectionConfig::Serial(SerialConfig {
        port: "/dev/null".to_string(),
        baud_rate: 115200,
        data_bits: DataBits::Eight,
        parity: Parity::None,
        stop_bits: StopBits::One,
        flow_control: FlowControl::None,
    });

    let _id = manager
        .create_connection("test".to_string(), connection_config)
        .await
        .unwrap();

    let stats = manager.stats().await;
    assert_eq!(stats.active_connections, 1);
}
