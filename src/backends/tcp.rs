//! TCP connection to a remote GDB stub (OpenOCD, probe-rs, pyOCD, etc.).

use crate::config::ProxyConfig;
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;

/// Establish a TCP connection to `target_host:target_port` (no codec).
pub async fn connect_tcp_stream(config: &ProxyConfig) -> Result<TcpStream, RsgdbError> {
    let backend_addr = format!("{}:{}", config.target_host, config.target_port);
    if config.timeout_secs == 0 {
        TcpStream::connect(&backend_addr)
            .await
            .map_err(RsgdbError::Io)
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
        .map_err(RsgdbError::Io)
    }
}

/// Connect to `target_host:target_port` and wrap the stream with [`GdbCodec`].
pub async fn connect_tcp_backend(
    config: &ProxyConfig,
) -> Result<Framed<TcpStream, GdbCodec>, RsgdbError> {
    let stream = connect_tcp_stream(config).await?;
    Ok(Framed::new(stream, GdbCodec::new()))
}
