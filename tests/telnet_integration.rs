use ebterm::connection::{
    Connection, ConnectionError, ConnectionStatus, TelnetConfig, TelnetConnection,
};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Mock Telnet server for testing.
/// Accepts connections, sends greeting, echoes data back.
struct MockTelnetServer {
    listener: TcpListener,
}

impl MockTelnetServer {
    async fn new() -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await.expect("Failed to bind");
        Self { listener }
    }

    fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().expect("Failed to get local addr")
    }

    async fn accept_and_handle(&self) {
        let (mut stream, _) = self.listener.accept().await.expect("Failed to accept");

        stream
            .write_all(b"MockTelnetServer Ready\r\n")
            .await
            .expect("Failed to write greeting");

        let mut buffer = [0u8; 1024];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    if stream.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

// --- Config tests ---

#[test]
fn test_telnet_config_creation() {
    let config = TelnetConfig {
        host: "192.168.1.100".to_string(),
        port: 23,
        connect_timeout_secs: 10,
    };

    assert_eq!(config.host, "192.168.1.100");
    assert_eq!(config.port, 23);
    assert_eq!(config.connect_timeout_secs, 10);
}

#[test]
fn test_telnet_config_default() {
    let config = TelnetConfig::default();

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 23);
    assert_eq!(config.connect_timeout_secs, 30);
}

// --- TelnetConnection creation tests ---

#[test]
fn test_telnet_connection_creation() {
    let config = TelnetConfig {
        host: "192.168.1.1".to_string(),
        port: 23,
        connect_timeout_secs: 10,
    };

    let conn = TelnetConnection::new(config);

    assert_eq!(
        conn.connection_type(),
        ebterm::connection::ConnectionType::Telnet
    );
    assert_eq!(conn.status(), ConnectionStatus::Disconnected);
    assert!(!conn.is_connected());
}

// --- Negative tests (MUST HAVE) ---

#[tokio::test]
async fn test_write_while_disconnected() {
    let config = TelnetConfig::default();
    let mut conn = TelnetConnection::new(config);

    let result = conn.write(b"data").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectionError::NotConnected => {}
        other => panic!("Expected NotConnected, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_read_while_disconnected() {
    let config = TelnetConfig::default();
    let mut conn = TelnetConnection::new(config);

    let mut buf = [0u8; 64];
    let result = conn.read(&mut buf).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectionError::NotConnected => {}
        other => panic!("Expected NotConnected, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_connection_refused() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    // Don't accept any connections - just get the port, then drop server
    drop(server);

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 2,
    };

    let mut conn = TelnetConnection::new(config);
    let result = conn.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connection_timeout() {
    // Use a non-routable IP (RFC 5737 TEST-NET)
    let config = TelnetConfig {
        host: "192.0.2.1".to_string(),
        port: 9999,
        connect_timeout_secs: 1,
    };

    let mut conn = TelnetConnection::new(config);
    let result = conn.connect().await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectionError::Timeout { .. } => {}
        other => panic!("Expected Timeout, got: {:?}", other),
    }
}

// --- Happy path tests ---

#[tokio::test]
async fn test_telnet_connect_disconnect() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("Failed to connect");

    assert!(conn.is_connected());
    assert_eq!(conn.status(), ConnectionStatus::Connected);

    // Read greeting
    let mut buf = [0u8; 64];
    let n = conn.read(&mut buf).await.expect("Failed to read greeting");
    let greeting = String::from_utf8_lossy(&buf[..n]);
    assert!(greeting.contains("MockTelnetServer Ready"));

    conn.disconnect().await.expect("Failed to disconnect");
    assert!(!conn.is_connected());
    assert_eq!(conn.status(), ConnectionStatus::Disconnected);

    server_handle.abort();
}

#[tokio::test]
async fn test_telnet_write_read_echo() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("Failed to connect");

    // Read greeting first
    let mut buf = [0u8; 1024];
    conn.read(&mut buf).await.expect("Failed to read greeting");

    // Send data and read echo
    let test_data = b"Hello, Telnet Server!";
    conn.write(test_data).await.expect("Failed to write");

    let n = conn.read(&mut buf).await.expect("Failed to read echo");
    assert_eq!(&buf[..n], test_data);

    conn.disconnect().await.ok();
    server_handle.abort();
}

#[tokio::test]
async fn test_double_connect_is_idempotent() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("First connect failed");

    // Second connect should return Ok (idempotent)
    conn.connect().await.expect("Second connect should be Ok");

    assert!(conn.is_connected());

    conn.disconnect().await.ok();
    server_handle.abort();
}

// --- Stats tests (SHOULD HAVE) ---

#[tokio::test]
async fn test_stats_accumulation() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("Failed to connect");

    // Initial stats should be zero (except connected_at)
    let stats = conn.stats();
    assert_eq!(stats.bytes_sent, 0);
    assert_eq!(stats.bytes_received, 0);
    assert!(stats.connected_at.is_some());

    // Read greeting
    let mut buf = [0u8; 1024];
    let n = conn.read(&mut buf).await.expect("Failed to read greeting");
    assert!(conn.stats().bytes_received > 0);
    assert_eq!(conn.stats().bytes_received, n as u64);

    // Write data
    let test_data = b"test";
    conn.write(test_data).await.expect("Failed to write");
    assert_eq!(conn.stats().bytes_sent, test_data.len() as u64);

    // Read echo
    conn.read(&mut buf).await.expect("Failed to read echo");
    assert!(conn.stats().bytes_received > n as u64);

    // Clear stats
    conn.clear_stats();
    let stats = conn.stats();
    assert_eq!(stats.bytes_sent, 0);
    assert_eq!(stats.bytes_received, 0);

    conn.disconnect().await.ok();
    server_handle.abort();
}

// --- Large payload test (SHOULD HAVE) ---

#[tokio::test]
async fn test_large_payload() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("Failed to connect");

    // Read greeting
    let mut buf = [0u8; 1024];
    conn.read(&mut buf).await.expect("Failed to read greeting");

    // Send large payload (4KB)
    let large_data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    conn.write(&large_data)
        .await
        .expect("Failed to write large payload");

    // Read echo (may come in chunks)
    let mut received = Vec::new();
    while received.len() < large_data.len() {
        let n = conn.read(&mut buf).await.expect("Failed to read");
        received.extend_from_slice(&buf[..n]);
    }

    assert_eq!(received, large_data);

    conn.disconnect().await.ok();
    server_handle.abort();
}

// --- Server disconnect test (SHOULD HAVE) ---

#[tokio::test]
async fn test_server_disconnect_during_read() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    // Server accepts one connection then closes it
    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = server.listener.accept().await.expect("Failed to accept");
        stream.write_all(b"Hello\r\n").await.ok();
        // Server closes connection
    });

    let config = TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = TelnetConnection::new(config);
    conn.connect().await.expect("Failed to connect");

    // Read greeting
    let mut buf = [0u8; 64];
    conn.read(&mut buf).await.expect("Failed to read greeting");

    // Wait for server to close
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Read should return NotConnected (server closed)
    let result = conn.read(&mut buf).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ConnectionError::NotConnected => {}
        other => panic!("Expected NotConnected after server close, got: {:?}", other),
    }

    server_handle.await.ok();
}
