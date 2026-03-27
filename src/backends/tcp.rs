//! TCP connection to a remote GDB stub (OpenOCD, probe-rs, pyOCD, etc.).
//!
//! Native probe backends will plug in alongside this path; see [#9](https://github.com/DynamicDevices/rsgdb/issues/9).

use crate::config::ProxyConfig;
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

/// Connect to `target_host:target_port` and wrap the stream with [`GdbCodec`].
pub async fn connect_tcp_backend(
    config: &ProxyConfig,
) -> Result<Framed<TcpStream, GdbCodec>, RsgdbError> {
    let backend_addr = format!("{}:{}", config.target_host, config.target_port);
    let stream = if config.timeout_secs == 0 {
        TcpStream::connect(&backend_addr)
            .await
            .map_err(RsgdbError::Io)?
    } else {
        timeout(
            Duration::from_secs(config.timeout_secs),
            TcpStream::connect(backend_addr.clone()),
        )
        .await
        .map_err(|_| {
            RsgdbError::Timeout(format!(
                "TCP connect to backend {} exceeded {}s",
                backend_addr, config.timeout_secs
            ))
        })?
        .map_err(RsgdbError::Io)?
    };
    Ok(Framed::new(stream, GdbCodec::new()))
}
