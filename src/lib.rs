//! # rsgdb - Enhanced GDB Server/Proxy
//!
//! A modern, feature-rich GDB server/proxy written in Rust for embedded debugging.
//!
//! ## Features
//!
//! - **Enhanced Logging**: Comprehensive protocol traffic logging with structured output
//! - **Advanced Breakpoints**: Named, conditional, and grouped breakpoint management
//! - **State Tracking**: Memory snapshots, register tracking, and peripheral decoding
//! - **Session Recording**: Record and replay debugging sessions
//! - **Backend Flexibility**: Support for multiple debug probes
//!
//! ## Example
//!
//! ```no_run
//! use rsgdb::proxy::ProxyConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ProxyConfig::default();
//! // Start the proxy server
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod error;
pub mod proxy;
pub mod protocol;
pub mod breakpoints;
pub mod state;
pub mod logger;
pub mod backends;
pub mod recorder;
pub mod ui;

// Re-export commonly used types
pub use config::Config;
pub use error::{RsgdbError, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}

// Made with Bob
