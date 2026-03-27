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
//! use rsgdb::config::ProxyConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let _proxy = ProxyConfig::default();
//! // Start the proxy with `ProxyServer::new` when wiring your binary
//! # Ok(())
//! # }
//! ```

pub mod backends;
pub mod breakpoints;
pub mod config;
pub mod error;
pub mod logger;
pub mod protocol;
pub mod proxy;
pub mod recorder;
pub mod state;
pub mod ui;

// Re-export commonly used types
pub use config::Config;
pub use error::{Result, RsgdbError};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}

// Made with Bob
