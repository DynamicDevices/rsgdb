//! Debug probe backends: how the proxy reaches the **target** GDB remote.
//!
//! - **[`BackendTransport`]** — `tcp` (default): connect to a GDB stub on `proxy.target_host` /
//!   `proxy.target_port`. `native` is reserved for direct probe integration ([#9](https://github.com/DynamicDevices/rsgdb/issues/9)).
//! - **`backend_type`** in [`crate::config::BackendConfig`] is a **label** (openocd, probe-rs, …)
//!   for logging and future tooling; it does not change the wire path today.

mod stream;
mod tcp;

pub use stream::BackendStream;
pub use tcp::{connect_tcp_backend, connect_tcp_stream};

use crate::config::{BackendConfig, BackendTransport, ProxyConfig};
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use tokio_util::codec::Framed;

/// Connect to the configured debug backend and return an RSP-framed stream.
pub async fn connect_backend(
    proxy: &ProxyConfig,
    backend: &BackendConfig,
) -> Result<Framed<BackendStream, GdbCodec>, RsgdbError> {
    match backend.transport {
        BackendTransport::Tcp => {
            let stream = connect_tcp_stream(proxy).await?;
            Ok(Framed::new(BackendStream::Tcp(stream), GdbCodec::new()))
        }
        BackendTransport::Native => Err(RsgdbError::NotImplemented(
            "native probe backend is not implemented yet; use [backend] transport = \"tcp\" \
             (GDB stub on target_host:target_port). See GitHub #9."
                .to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BackendConfig;

    #[tokio::test]
    async fn native_transport_is_not_implemented() {
        let proxy = ProxyConfig::default();
        let backend = BackendConfig {
            transport: BackendTransport::Native,
            ..Default::default()
        };
        let err = connect_backend(&proxy, &backend).await.unwrap_err();
        match err {
            RsgdbError::NotImplemented(s) => assert!(s.contains("native")),
            e => panic!("expected NotImplemented, got {e:?}"),
        }
    }
}
