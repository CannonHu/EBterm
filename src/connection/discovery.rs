//! Serial port discovery

use serde::{Deserialize, Serialize};

/// Discovered serial port information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPort {
    pub port_name: String,
    pub port_type: String,
}

/// Error during port discovery
#[derive(Debug, Clone)]
pub enum DiscoveryError {
    EnumerationFailed(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::EnumerationFailed(msg) => {
                write!(f, "Failed to enumerate ports: {msg}")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Discover available serial ports
pub fn discover_serial_ports() -> Result<Vec<DiscoveredPort>, DiscoveryError> {
    match serial2_tokio::SerialPort::available_ports() {
        Ok(ports) => {
            let discovered = ports
                .into_iter()
                .map(|path| {
                    let port_name = path.to_string_lossy().to_string();
                    DiscoveredPort {
                        port_name,
                        port_type: "serial".to_string(),
                    }
                })
                .collect();
            Ok(discovered)
        }
        Err(e) => Err(DiscoveryError::EnumerationFailed(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_port_creation() {
        let port = DiscoveredPort {
            port_name: "/dev/ttyUSB0".to_string(),
            port_type: "serial".to_string(),
        };
        assert_eq!(port.port_name, "/dev/ttyUSB0");
        assert_eq!(port.port_type, "serial");
    }

    #[test]
    fn test_discovery_error_display() {
        let err = DiscoveryError::EnumerationFailed("test error".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to enumerate ports: test error"
        );
    }
}
