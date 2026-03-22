//! Serial connection implementation
//!
//! Provides serial port communication using serial2-tokio

use async_trait::async_trait;
use std::path::Path;
use tokio::io::AsyncWriteExt;

use super::{Connection, ConnectionError, ConnectionStatus, ConnectionStats, ConnectionType};
use super::types::{DataBits, FlowControl, Parity, SerialConfig, StopBits};

/// Serial connection implementation
#[derive(Debug)]
pub struct SerialConnection {
    config: SerialConfig,
    status: ConnectionStatus,
    stats: ConnectionStats,
    port: Option<serial2_tokio::SerialPort>,
}

impl SerialConnection {
    pub fn new(config: SerialConfig) -> Self {
        Self {
            config,
            status: ConnectionStatus::Disconnected,
            stats: ConnectionStats::default(),
            port: None,
        }
    }

    pub fn config(&self) -> &SerialConfig {
        &self.config
    }

    fn to_serial_char_size(bits: DataBits) -> serial2_tokio::CharSize {
        match bits {
            DataBits::Seven => serial2_tokio::CharSize::Bits7,
            DataBits::Eight => serial2_tokio::CharSize::Bits8,
        }
    }

    fn to_serial_parity(parity: Parity) -> serial2_tokio::Parity {
        match parity {
            Parity::None => serial2_tokio::Parity::None,
            Parity::Odd => serial2_tokio::Parity::Odd,
            Parity::Even => serial2_tokio::Parity::Even,
        }
    }

    fn to_serial_stop_bits(bits: StopBits) -> serial2_tokio::StopBits {
        match bits {
            StopBits::One => serial2_tokio::StopBits::One,
            StopBits::Two => serial2_tokio::StopBits::Two,
        }
    }

    fn to_serial_flow_control(flow: FlowControl) -> serial2_tokio::FlowControl {
        match flow {
            FlowControl::None => serial2_tokio::FlowControl::None,
            FlowControl::Software => serial2_tokio::FlowControl::XonXoff,
            FlowControl::Hardware => serial2_tokio::FlowControl::RtsCts,
        }
    }
}

#[async_trait]
impl Connection for SerialConnection {
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Serial
    }

    fn status(&self) -> ConnectionStatus {
        self.status
    }

    async fn connect(&mut self) -> Result<(), ConnectionError> {
        if self.status == ConnectionStatus::Connected {
            return Ok(());
        }

        self.status = ConnectionStatus::Connecting;

        let path = Path::new(&self.config.port);
        let data_bits = self.config.data_bits;
        let parity = self.config.parity;
        let stop_bits = self.config.stop_bits;
        let flow_control = self.config.flow_control;
        let baud_rate = self.config.baud_rate;

        let port = serial2_tokio::SerialPort::open(
            path,
            move |mut settings: serial2_tokio::Settings| -> std::io::Result<serial2_tokio::Settings> {
                settings.set_raw();
                settings.set_baud_rate(baud_rate)?;
                settings.set_char_size(Self::to_serial_char_size(data_bits));
                settings.set_parity(Self::to_serial_parity(parity));
                settings.set_stop_bits(Self::to_serial_stop_bits(stop_bits));
                settings.set_flow_control(Self::to_serial_flow_control(flow_control));
                Ok(settings)
            },
        ).map_err(|e| ConnectionError::Serial(format!("Failed to open port: {}", e)))?;

        self.port = Some(port);
        self.status = ConnectionStatus::Connected;
        self.stats.connected_at = Some(std::time::Instant::now());

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ConnectionError> {
        if let Some(port) = self.port.take() {
            drop(port);
        }

        self.status = ConnectionStatus::Disconnected;
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError> {
        let port = self.port.as_ref()
            .ok_or(ConnectionError::NotConnected)?;

        match port.read(buf).await {
            Ok(n) => {
                self.stats.bytes_received += n as u64;
                self.stats.packets_received += 1;
                Ok(n)
            }
            Err(e) => Err(ConnectionError::Io(e)),
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        let port = self.port.as_mut()
            .ok_or(ConnectionError::NotConnected)?;

        match port.write(buf).await {
            Ok(n) => {
                self.stats.bytes_sent += n as u64;
                self.stats.packets_sent += 1;
                Ok(n)
            }
            Err(e) => Err(ConnectionError::Io(e)),
        }
    }

    async fn flush(&mut self) -> Result<(), ConnectionError> {
        if let Some(port) = self.port.as_mut() {
            port.flush().await
                .map_err(ConnectionError::Io)?;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.status == ConnectionStatus::Connected && self.port.is_some()
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
    fn test_serial_connection_creation() {
        let config = SerialConfig {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            flow_control: FlowControl::None,
        };

        let conn = SerialConnection::new(config);

        assert_eq!(conn.connection_type(), ConnectionType::Serial);
        assert_eq!(conn.status(), ConnectionStatus::Disconnected);
        assert!(!conn.is_connected());
        assert_eq!(conn.config().port, "/dev/ttyUSB0");
    }

    #[test]
    fn test_serial_data_bits_conversion() {
        assert_eq!(
            SerialConnection::to_serial_char_size(DataBits::Seven),
            serial2_tokio::CharSize::Bits7
        );
        assert_eq!(
            SerialConnection::to_serial_char_size(DataBits::Eight),
            serial2_tokio::CharSize::Bits8
        );
    }

    #[test]
    fn test_serial_parity_conversion() {
        assert_eq!(
            SerialConnection::to_serial_parity(Parity::None),
            serial2_tokio::Parity::None
        );
        assert_eq!(
            SerialConnection::to_serial_parity(Parity::Odd),
            serial2_tokio::Parity::Odd
        );
        assert_eq!(
            SerialConnection::to_serial_parity(Parity::Even),
            serial2_tokio::Parity::Even
        );
    }

    #[test]
    fn test_serial_stop_bits_conversion() {
        assert_eq!(
            SerialConnection::to_serial_stop_bits(StopBits::One),
            serial2_tokio::StopBits::One
        );
        assert_eq!(
            SerialConnection::to_serial_stop_bits(StopBits::Two),
            serial2_tokio::StopBits::Two
        );
    }

    #[test]
    fn test_serial_flow_control_conversion() {
        assert_eq!(
            SerialConnection::to_serial_flow_control(FlowControl::None),
            serial2_tokio::FlowControl::None
        );
        assert_eq!(
            SerialConnection::to_serial_flow_control(FlowControl::Software),
            serial2_tokio::FlowControl::XonXoff
        );
        assert_eq!(
            SerialConnection::to_serial_flow_control(FlowControl::Hardware),
            serial2_tokio::FlowControl::RtsCts
        );
    }

    #[test]
    fn test_serial_connection_stats() {
        let config = SerialConfig::default();
        let mut conn = SerialConnection::new(config);

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
