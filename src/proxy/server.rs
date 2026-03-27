//! Proxy server implementation
//!
//! Handles incoming GDB client connections and forwards commands to the backend.

use crate::config::{ProxyConfig, RecordingConfig};
use crate::error::RsgdbError;
use crate::protocol::codec::{GdbCodec, PacketOrAck};
use crate::protocol::commands::{GdbCommand, QueryCommand};
use crate::recorder::{RecordDirection, RecordEventV1, SessionRecorder};
use crate::rtos;
use crate::svd::SvdIndex;
use futures::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

/// Proxy server that bridges GDB clients and debug backends
pub struct ProxyServer {
    config: ProxyConfig,
    recording: RecordingConfig,
    svd: Option<Arc<SvdIndex>>,
    listener: TcpListener,
}

impl ProxyServer {
    /// Create a new proxy server
    pub async fn new(
        config: ProxyConfig,
        recording: RecordingConfig,
        svd: Option<Arc<SvdIndex>>,
    ) -> Result<Self, RsgdbError> {
        let addr = format!("0.0.0.0:{}", config.listen_port);
        info!("Starting proxy server on {}", addr);

        let listener = TcpListener::bind(&addr).await.map_err(RsgdbError::Io)?;

        info!("Proxy server listening on {}", addr);

        Ok(Self {
            config,
            recording,
            svd,
            listener,
        })
    }

    /// Local socket address this server is bound to (useful when `listen_port` is `0`).
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Run the proxy server
    pub async fn run(&mut self) -> Result<(), RsgdbError> {
        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New connection from {}", addr);
                    let config = self.config.clone();
                    let recording = self.recording.clone();
                    let svd = self.svd.clone();

                    // Spawn a new task to handle this connection
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, config, recording, svd).await {
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
    recording: RecordingConfig,
    svd: Option<Arc<SvdIndex>>,
) -> Result<(), RsgdbError> {
    let peer_addr = client_socket.peer_addr().map_err(RsgdbError::Io)?;
    info!("Handling connection from {}", peer_addr);

    // Connect to the backend
    let backend_addr = format!("{}:{}", config.target_host, config.target_port);
    debug!("Connecting to backend at {}", backend_addr);

    let backend_socket = if config.timeout_secs == 0 {
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

    info!("Connected to backend at {}", backend_addr);

    let recorder = if recording.enabled {
        match SessionRecorder::create(&recording).await {
            Ok(r) => Some(r),
            Err(e) => {
                error!("Failed to start session recording: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create a session to manage the connection
    let mut session = ProxySession::new(client_socket, backend_socket, config, recorder, svd);

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
    recorder: Option<SessionRecorder>,
    svd: Option<Arc<SvdIndex>>,
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
    fn new(
        client_socket: TcpStream,
        backend_socket: TcpStream,
        config: ProxyConfig,
        recorder: Option<SessionRecorder>,
        svd: Option<Arc<SvdIndex>>,
    ) -> Self {
        let client = Framed::new(client_socket, GdbCodec::new());
        let backend = Framed::new(backend_socket, GdbCodec::new());

        Self {
            client,
            backend,
            config,
            stats: Arc::new(Mutex::new(SessionStats::default())),
            recorder,
            svd,
        }
    }

    async fn flush_recording(&mut self) {
        if let Some(ref mut r) = self.recorder {
            if let Err(e) = r.flush().await {
                error!("recording flush failed: {}", e);
            }
        }
    }

    /// Run the proxy session
    async fn run(&mut self) -> Result<(), RsgdbError> {
        let res = self.run_inner().await;
        self.flush_recording().await;
        res
    }

    async fn run_inner(&mut self) -> Result<(), RsgdbError> {
        debug!(
            "Session settings: enable_acks={}, timeout_secs={}",
            self.config.enable_acks, self.config.timeout_secs
        );
        if !self.config.enable_acks {
            warn!(
                "proxy.enable_acks is false: RSP +/- bytes are still forwarded; use GDB \
                 `set remote noack-packet` / `set remote interrupt-sequence` for protocol-level ack behavior"
            );
        }

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

    async fn record_trace(&mut self, direction: RecordDirection, item: &PacketOrAck) {
        if let Some(ref mut r) = self.recorder {
            let ev = RecordEventV1::from_rsp(direction, item);
            if let Err(e) = r.record(&ev).await {
                error!("recording write failed: {}", e);
            }
        }
    }

    /// Log SVD annotation for client memory RSP (`m` / `M`) when an index is configured.
    fn log_svd_client_packet(&self, item: &PacketOrAck) {
        let Some(ref idx) = self.svd else {
            return;
        };
        let PacketOrAck::Packet(p) = item else {
            return;
        };
        let Ok(cmd) = GdbCommand::parse(&p.data) else {
            return;
        };
        match cmd {
            GdbCommand::ReadMemory { addr, len } => {
                if let Some(note) = idx.annotate_access(addr, len as u64) {
                    tracing::debug!(
                        target: "rsgdb::svd",
                        direction = "client_to_backend",
                        rsp = "m",
                        addr = format!("0x{addr:x}"),
                        len,
                        %note,
                        "memory read"
                    );
                }
            }
            GdbCommand::WriteMemory { addr, data } => {
                let len = data.len() as u64;
                if let Some(note) = idx.annotate_access(addr, len) {
                    tracing::debug!(
                        target: "rsgdb::svd",
                        direction = "client_to_backend",
                        rsp = "M",
                        addr = format!("0x{addr:x}"),
                        len,
                        %note,
                        "memory write"
                    );
                }
            }
            _ => {}
        }
    }

    /// Decode/log GDB thread-extension RSP (Zephyr, FreeRTOS, … — stub-dependent). `target: rsgdb::rtos`.
    fn log_rtos_client_packet(&self, item: &PacketOrAck) {
        let PacketOrAck::Packet(p) = item else {
            return;
        };
        let Ok(cmd) = GdbCommand::parse(&p.data) else {
            return;
        };
        match cmd {
            GdbCommand::Query(QueryCommand::CurrentThread) => {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    kind = "qC",
                    "current thread query"
                );
            }
            GdbCommand::Query(QueryCommand::FirstThreadInfo) => {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    kind = "qfThreadInfo",
                    "thread list (first)"
                );
            }
            GdbCommand::Query(QueryCommand::SubsequentThreadInfo) => {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    kind = "qsThreadInfo",
                    "thread list (next)"
                );
            }
            GdbCommand::Query(QueryCommand::ThreadExtraInfo(ref id)) => {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    kind = "qThreadExtraInfo",
                    thread_id_hex = %id,
                    "thread name query"
                );
            }
            GdbCommand::SetThread {
                for_continue,
                thread_id,
            } => {
                let op = if for_continue { "Hc" } else { "Hg" };
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    op,
                    thread_id,
                    "set thread"
                );
            }
            GdbCommand::Query(QueryCommand::Other(ref s)) if s.starts_with("Xfer:threads") => {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "client_to_backend",
                    kind = "qXfer:threads",
                    "thread XML read"
                );
            }
            _ => {}
        }
    }

    fn log_rtos_backend_packet(&self, item: &PacketOrAck) {
        let PacketOrAck::Packet(p) = item else {
            return;
        };
        if p.data.first() == Some(&b'T') {
            if let Some(note) = rtos::summarize_stop_reply(&p.data) {
                tracing::debug!(
                    target: "rsgdb::rtos",
                    direction = "backend_to_client",
                    summary = %note,
                    "stop reply"
                );
            }
        } else if let Some(note) = rtos::summarize_backend_thread_payload(&p.data) {
            tracing::debug!(
                target: "rsgdb::rtos",
                direction = "backend_to_client",
                summary = %note,
                "backend thread reply"
            );
        }
    }

    /// Handle data received from the client
    async fn handle_client_data(&mut self, data: PacketOrAck) -> Result<(), RsgdbError> {
        self.log_svd_client_packet(&data);
        self.log_rtos_client_packet(&data);
        self.record_trace(RecordDirection::ClientToBackend, &data)
            .await;

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
        self.record_trace(RecordDirection::BackendToClient, &data)
            .await;
        self.log_rtos_backend_packet(&data);

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

        let result =
            ProxyServer::new(config, crate::config::RecordingConfig::default(), None).await;
        assert!(result.is_ok());
    }
}

// Made with Bob
