mod common;

use embedded_debugger::connection::types::{Connection, ConnectionStatus};
use embedded_debugger::connection::ConnectionError;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use common::MockConnection;

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
            .write_all(b"MockServer Ready\r\n")
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

pub async fn assert_connection_connect_disconnect_cycle<C: Connection + Default + Send>(
) -> Result<(), ConnectionError> {
    let mut conn = C::default();

    assert!(
        !conn.is_connected(),
        "Initial state should be disconnected"
    );
    assert_eq!(
        conn.status(),
        ConnectionStatus::Disconnected,
        "Initial status should be Disconnected"
    );

    conn.connect().await?;

    assert!(
        conn.is_connected(),
        "After connect, should be connected"
    );
    assert_eq!(
        conn.status(),
        ConnectionStatus::Connected,
        "After connect, status should be Connected"
    );

    conn.disconnect().await?;

    assert!(
        !conn.is_connected(),
        "After disconnect, should be disconnected"
    );
    assert_eq!(
        conn.status(),
        ConnectionStatus::Disconnected,
        "After disconnect, status should be Disconnected"
    );

    Ok(())
}

pub async fn assert_connection_write_read_cycle<C: Connection + Default + Send>(
    test_data: &[u8],
) -> Result<(), ConnectionError> {
    let mut conn = C::default();
    conn.connect().await?;

    let written_len = conn.write(test_data).await?;
    assert_eq!(
        written_len, test_data.len(),
        "Write should return correct length"
    );

    let stats = conn.stats();
    assert_eq!(
        stats.bytes_sent, test_data.len() as u64,
        "Stats should track bytes sent"
    );
    assert_eq!(
        stats.packets_sent, 1,
        "Stats should track packets sent"
    );

    conn.disconnect().await?;

    Ok(())
}

pub async fn assert_connection_stats_tracking<C: Connection + Default + Send>(
) -> Result<(), ConnectionError> {
    let mut conn = C::default();

    let initial_stats = conn.stats();
    assert_eq!(
        initial_stats.bytes_sent, 0,
        "Initial bytes_sent should be 0"
    );
    assert_eq!(
        initial_stats.bytes_received, 0,
        "Initial bytes_received should be 0"
    );

    conn.connect().await?;

    let connected_stats = conn.stats();
    assert!(
        connected_stats.connected_at.is_some(),
        "After connect, connected_at should be set"
    );

    conn.disconnect().await?;

    let disconnected_stats = conn.stats();
    assert!(
        disconnected_stats.connected_at.is_none(),
        "After disconnect, connected_at should be None"
    );

    Ok(())
}

pub async fn assert_connection_error_handling<C: Connection + Default + Send>(
) -> Result<(), ConnectionError> {
    let mut conn = C::default();

    let write_result = conn.write(b"test").await;
    assert!(
        write_result.is_err(),
        "Write while disconnected should return error"
    );

    let mut buf = [0u8; 10];
    let read_result = conn.read(&mut buf).await;
    assert!(
        read_result.is_err(),
        "Read while disconnected should return error"
    );

    Ok(())
}

pub async fn assert_connection_status_consistency<C: Connection + Default + Send>(
) -> Result<(), ConnectionError> {
    let mut conn = C::default();

    let is_connected = conn.is_connected();
    let status = conn.status();
    assert_eq!(
        is_connected,
        status == ConnectionStatus::Connected,
        "is_connected() should match status() == Connected when disconnected"
    );

    conn.connect().await?;

    let is_connected = conn.is_connected();
    let status = conn.status();
    assert_eq!(
        is_connected, true,
        "is_connected() should be true when connected"
    );
    assert_eq!(
        status, ConnectionStatus::Connected,
        "status() should be Connected after connect"
    );
    assert!(
        is_connected,
        "is_connected() should match status() == Connected when connected"
    );

    conn.disconnect().await?;

    let is_connected = conn.is_connected();
    let status = conn.status();
    assert_eq!(
        is_connected, false,
        "is_connected() should be false when disconnected"
    );
    assert_eq!(
        status, ConnectionStatus::Disconnected,
        "status() should be Disconnected after disconnect"
    );

    Ok(())
}

#[tokio::test]
async fn test_mock_connection_contract() {
    assert_connection_connect_disconnect_cycle::<MockConnection>()
        .await
        .expect("connect-disconnect cycle failed");

    assert_connection_write_read_cycle::<MockConnection>(b"Hello, Contract!")
        .await
        .expect("write-read cycle failed");

    assert_connection_stats_tracking::<MockConnection>()
        .await
        .expect("stats tracking failed");

    assert_connection_error_handling::<MockConnection>()
        .await
        .expect("error handling failed");

    assert_connection_status_consistency::<MockConnection>()
        .await
        .expect("status consistency failed");
}

#[tokio::test]
async fn test_telnet_connection_contract() {
    let server = MockTelnetServer::new().await;
    let addr = server.local_addr();

    let server_handle = tokio::spawn(async move {
        server.accept_and_handle().await;
    });

    let config = embedded_debugger::connection::TelnetConfig {
        host: "127.0.0.1".to_string(),
        port: addr.port(),
        connect_timeout_secs: 5,
    };

    let mut conn = embedded_debugger::connection::TelnetConnection::new(config);

    assert!(
        !conn.is_connected(),
        "Initial state should be disconnected"
    );

    conn.connect().await.expect("Failed to connect");

    assert!(
        conn.is_connected(),
        "After connect, should be connected"
    );

    let mut buf = [0u8; 64];
    let n = conn
        .read(&mut buf)
        .await
        .expect("Failed to read greeting");
    let greeting = String::from_utf8_lossy(&buf[..n]);
    assert!(
        greeting.contains("MockServer Ready"),
        "Should receive greeting"
    );

    let test_data = b"Contract Test Data";
    conn.write(test_data).await.expect("Failed to write");

    let n = conn.read(&mut buf).await.expect("Failed to read echo");
    assert_eq!(
        &buf[..n], test_data,
        "Echo should match sent data"
    );

    let stats = conn.stats();
    assert!(
        stats.bytes_sent > 0,
        "Stats should track sent bytes"
    );
    assert!(
        stats.bytes_received > 0,
        "Stats should track received bytes"
    );

    conn.disconnect().await.expect("Failed to disconnect");

    assert!(
        !conn.is_connected(),
        "After disconnect, should be disconnected"
    );

    server_handle.abort();
}