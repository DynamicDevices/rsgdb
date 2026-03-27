//! Debug probe backends.
//!
//! Today the proxy uses [`tcp::connect_tcp_backend`] to reach a GDB stub over TCP. A fuller
//! backend abstraction for native probes is tracked as
//! [#9](https://github.com/DynamicDevices/rsgdb/issues/9).

mod tcp;

pub use tcp::connect_tcp_backend;
