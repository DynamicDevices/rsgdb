//! Debug probe backends: how the proxy reaches the **target** GDB remote.
//!
//! - **[`BackendTransport`]** — `tcp` (default): connect to a GDB stub on `proxy.target_host` /
//!   `proxy.target_port`. `native`: spawn a stub with `[backend.spawn] program` and `{port}`, then
//!   TCP to `bind_host:port` ([#9](https://github.com/DynamicDevices/rsgdb/issues/9)).
//! - **`backend_type`** in [`crate::config::BackendConfig`] is a **label** (openocd, probe-rs, …)
//!   for logging and future tooling; it does not change the wire path today.

mod native;
mod stream;
mod tcp;

pub use stream::BackendStream;
pub use tcp::{connect_tcp_backend, connect_tcp_stream};

use crate::config::{BackendConfig, BackendTransport, ProxyConfig};
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use tokio_util::codec::Framed;

/// Result of [`connect_backend`]: framed RSP stream and an optional **managed** stub child process.
pub struct BackendConnection {
    pub framed: Framed<BackendStream, GdbCodec>,
    pub spawned_child: Option<tokio::process::Child>,
}

/// Connect to the configured debug backend and return an RSP-framed stream (and optional subprocess).
pub async fn connect_backend(
    proxy: &ProxyConfig,
    backend: &BackendConfig,
) -> Result<BackendConnection, RsgdbError> {
    match backend.transport {
        BackendTransport::Tcp => {
            let stream = connect_tcp_stream(proxy).await?;
            Ok(BackendConnection {
                framed: Framed::new(BackendStream::Tcp(stream), GdbCodec::new()),
                spawned_child: None,
            })
        }
        BackendTransport::Native => {
            let (framed, child) = native::connect_native_managed(&backend.spawn).await?;
            Ok(BackendConnection {
                framed,
                spawned_child: Some(child),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BackendConfig, BackendSpawnConfig, BackendTransport};

    #[tokio::test]
    async fn native_transport_requires_valid_spawn_in_config() {
        // Empty program fails at connect (validation should catch this first in normal use).
        let proxy = ProxyConfig::default();
        let backend = BackendConfig {
            transport: BackendTransport::Native,
            spawn: BackendSpawnConfig::default(),
            ..Default::default()
        };
        let err = match connect_backend(&proxy, &backend).await {
            Err(e) => e,
            Ok(_) => panic!("expected error for empty native spawn"),
        };
        match err {
            RsgdbError::Backend(s) => assert!(s.contains("{port}") || s.contains("spawn")),
            RsgdbError::Io(_) => {}
            e => panic!("unexpected {e:?}"),
        }
    }
}
