//! Proxy server implementation
//!
//! Handles incoming GDB client connections and forwards commands to the backend.

use crate::config::ProxyConfig;
use crate::error::RsgdbError;
use crate::protocol::codec::{GdbCodec, PacketOrAck};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};

/// Proxy server that bridges GDB clients and debug backends
pub struct ProxyServer {
    config: ProxyConfig,
    listener: TcpListener,
}

impl ProxyServer {
    /// Create a new proxy server
    pub async fn new(config: ProxyConfig) -> Result<Self, RsgdbError> {
        let addr = format!("0.0.0.0:{}", config.listen_port);
        info!("Starting proxy server on {}", addr);

        let listener = TcpListener::bind(&addr).await.map_err(RsgdbError::Io)?;

        info!("Proxy server listening on {}", addr);

        Ok(Self { config, listener })
    }

    /// Run the proxy server
    pub async fn run(&mut self) -> Result<(), RsgdbError> {
        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New connection from {}", addr);
                    let config = self.config.clone();

                    // Spawn a new task to handle this connection
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, config).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single GDB client connection
async fn handle_connection(
    client_socket: TcpStream,
    config: ProxyConfig,
) -> Result<(), RsgdbError> {
    let peer_addr = client_socket.peer_addr().map_err(RsgdbError::Io)?;
    info!("Handling connection from {}", peer_addr);

    // Connect to the backend
    let backend_addr = format!("{}:{}", config.target_host, config.target_port);
    debug!("Connecting to backend at {}", backend_addr);

    let backend_socket = TcpStream::connect(&backend_addr)
        .await
        .map_err(RsgdbError::Io)?;

    info!("Connected to backend at {}", backend_addr);

    // Create a session to manage the connection
    let mut session = ProxySession::new(client_socket, backend_socket, config);

    // Run the session
    session.run().await?;

    info!("Connection from {} closed", peer_addr);
    Ok(())
}

/// A proxy session managing a single client-backend connection pair
struct ProxySession {
    client: Framed<TcpStream, GdbCodec>,
    backend: Framed<TcpStream, GdbCodec>,
    config: ProxyConfig,
    stats: Arc<Mutex<SessionStats>>,
}

/// Statistics for a proxy session (ack/nack counts are total forwarded in either direction)
#[derive(Debug, Default)]
struct SessionStats {
    packets_from_client: u64,
    packets_to_client: u64,
    packets_from_backend: u64,
    packets_to_backend: u64,
    acks_forwarded: u64,
    nacks_forwarded: u64,
}

impl ProxySession {
    /// Create a new proxy session
    fn new(client_socket: TcpStream, backend_socket: TcpStream, config: ProxyConfig) -> Self {
        let client = Framed::new(client_socket, GdbCodec::new());
        let backend = Framed::new(backend_socket, GdbCodec::new());

        Self {
            client,
            backend,
            config,
            stats: Arc::new(Mutex::new(SessionStats::default())),
        }
    }

    /// Run the proxy session
    async fn run(&mut self) -> Result<(), RsgdbError> {
        use futures::StreamExt;

        debug!(
            "Session settings: enable_acks={}, timeout_secs={}",
            self.config.enable_acks, self.config.timeout_secs
        );

        loop {
            tokio::select! {
                // Handle data from client
                result = self.client.next() => {
                    match result {
                        Some(Ok(packet_or_ack)) => {
                            self.handle_client_data(packet_or_ack).await?;
                        }
                        Some(Err(e)) => {
                            error!("Error reading from client: {}", e);
                            return Err(RsgdbError::Protocol(e));
                        }
                        None => {
                            info!("Client disconnected");
                            return Ok(());
                        }
                    }
                }

                // Handle data from backend
                result = self.backend.next() => {
                    match result {
                        Some(Ok(packet_or_ack)) => {
                            self.handle_backend_data(packet_or_ack).await?;
                        }
                        Some(Err(e)) => {
                            error!("Error reading from backend: {}", e);
                            return Err(RsgdbError::Protocol(e));
                        }
                        None => {
                            info!("Backend disconnected");
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Handle data received from the client
    async fn handle_client_data(&mut self, data: PacketOrAck) -> Result<(), RsgdbError> {
        let mut stats = self.stats.lock().await;

        match data {
            PacketOrAck::Packet(packet) => {
                stats.packets_from_client += 1;
                drop(stats);

                debug!("Client -> Backend: {:?}", packet);

                // Forward packet to backend
                use futures::SinkExt;
                self.backend
                    .send(PacketOrAck::Packet(packet))
                    .await
                    .map_err(RsgdbError::Protocol)?;

                let mut stats = self.stats.lock().await;
                stats.packets_to_backend += 1;
            }
            PacketOrAck::Ack => {
                debug!("Client -> Backend: ACK");

                // Forward ACK to backend
                use futures::SinkExt;
                self.backend
                    .send(PacketOrAck::Ack)
                    .await
                    .map_err(RsgdbError::Protocol)?;

                stats.acks_forwarded += 1;
            }
            PacketOrAck::Nack => {
                debug!("Client -> Backend: NACK");

                // Forward NACK to backend
                use futures::SinkExt;
                self.backend
                    .send(PacketOrAck::Nack)
                    .await
                    .map_err(RsgdbError::Protocol)?;

                stats.nacks_forwarded += 1;
            }
        }

        Ok(())
    }

    /// Handle data received from the backend
    async fn handle_backend_data(&mut self, data: PacketOrAck) -> Result<(), RsgdbError> {
        let mut stats = self.stats.lock().await;

        match data {
            PacketOrAck::Packet(packet) => {
                stats.packets_from_backend += 1;
                drop(stats);

                debug!("Backend -> Client: {:?}", packet);

                // Forward packet to client
                use futures::SinkExt;
                self.client
                    .send(PacketOrAck::Packet(packet))
                    .await
                    .map_err(RsgdbError::Protocol)?;

                let mut stats = self.stats.lock().await;
                stats.packets_to_client += 1;
            }
            PacketOrAck::Ack => {
                debug!("Backend -> Client: ACK");

                // Forward ACK to client
                use futures::SinkExt;
                self.client
                    .send(PacketOrAck::Ack)
                    .await
                    .map_err(RsgdbError::Protocol)?;

                stats.acks_forwarded += 1;
            }
            PacketOrAck::Nack => {
                debug!("Backend -> Client: NACK");

                // Forward NACK to client
                use futures::SinkExt;
                self.client
                    .send(PacketOrAck::Nack)
                    .await
                    .map_err(RsgdbError::Protocol)?;

                stats.nacks_forwarded += 1;
            }
        }

        Ok(())
    }

    /// Get session statistics
    #[allow(dead_code)]
    async fn get_stats(&self) -> SessionStats {
        let stats = self.stats.lock().await;
        SessionStats {
            packets_from_client: stats.packets_from_client,
            packets_to_client: stats.packets_to_client,
            packets_from_backend: stats.packets_from_backend,
            packets_to_backend: stats.packets_to_backend,
            acks_forwarded: stats.acks_forwarded,
            nacks_forwarded: stats.nacks_forwarded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_stats_default() {
        let stats = SessionStats::default();
        assert_eq!(stats.packets_from_client, 0);
        assert_eq!(stats.packets_to_client, 0);
        assert_eq!(stats.packets_from_backend, 0);
        assert_eq!(stats.packets_to_backend, 0);
        assert_eq!(stats.acks_forwarded, 0);
        assert_eq!(stats.nacks_forwarded, 0);
    }

    #[tokio::test]
    async fn test_proxy_server_creation() {
        let config = ProxyConfig {
            listen_port: 0,
            target_host: "localhost".to_string(),
            target_port: 3334,
            enable_acks: true,
            timeout_secs: 30,
        };

        let result = ProxyServer::new(config).await;
        assert!(result.is_ok());
    }
}

// Made with Bob
