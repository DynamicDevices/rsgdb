//! Proxy module
//!
//! Core proxy logic for bridging GDB and debug backends.

use serde::{Deserialize, Serialize};

/// Configuration for the proxy server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port to listen on for GDB connections
    pub listen_port: u16,

    /// Backend type (openocd, probe-rs, pyocd)
    pub backend: String,

    /// Target host
    pub target_host: String,

    /// Target port
    pub target_port: u16,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_port: 3333,
            backend: "openocd".to_string(),
            target_host: "localhost".to_string(),
            target_port: 3334,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert_eq!(config.listen_port, 3333);
        assert_eq!(config.backend, "openocd");
    }
}

// Made with Bob
