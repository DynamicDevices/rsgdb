//! Proxy module
//!
//! Core proxy logic for bridging GDB and debug backends.

mod server;

pub use crate::config::ProxyConfig;
pub use server::ProxyServer;
