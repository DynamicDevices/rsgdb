//! # rsgdb - Enhanced GDB Server/Proxy
//!
//! A modern, feature-rich GDB server/proxy written in Rust for embedded debugging.
//!
//! ## Features
//!
//! - **Logging**: Protocol traffic and optional SVD/RTOS decode logs (`tracing`)
//! - **Proxy**: TCP RSP forward between GDB and a stub (OpenOCD, probe-rs, etc.)
//! - **Recording / replay**: JSONL session capture and `replay` mock backend
//! - **SVD**: Read-only peripheral/register (and field) labels for memory packets in logs
//! - **Breakpoints / state / UI**: Lightweight modules and config for future work; proxy does not rewrite breakpoint RSP yet
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
pub mod flash;
pub mod logger;
mod logging_setup;
pub mod protocol;
pub mod proxy;
pub mod recorder;
pub mod replay;
pub mod rtos;
pub mod state;
pub mod svd;
pub mod ui;

// Re-export commonly used types
pub use config::{Config, FlashConfig, SvdConfig};
pub use error::{Result, RsgdbError};
pub use logging_setup::{init_from_logging_config, LoggingInitGuard};

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
