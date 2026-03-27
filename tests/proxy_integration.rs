//! End-to-end TCP tests: fake RSP backend echoes through the proxy (L1 / L2-style RSP smoke).
//!
//! These tests do **not** require a `gdb` binary; they exercise the same RSP framing GDB uses.

use futures::{SinkExt, StreamExt};
use rsgdb::config::{ProxyConfig, RecordingConfig};
use rsgdb::protocol::codec::{GdbCodec, PacketOrAck};
use rsgdb::protocol::Packet;
use rsgdb::proxy::ProxyServer;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;

/// `TcpListener` bound to `0.0.0.0` / `::` reports that as `local_addr()`. Connecting to
/// unspecified addresses works on Unix but fails on Windows (WSAEADDRNOTAVAIL). Use loopback
/// for same-host test clients.
fn connect_addr(listen: SocketAddr) -> SocketAddr {
    match listen {
        SocketAddr::V4(a) if a.ip().is_unspecified() => {
            SocketAddr::new(Ipv4Addr::LOCALHOST.into(), a.port())
        }
        SocketAddr::V6(a) if a.ip().is_unspecified() => {
            SocketAddr::new(Ipv6Addr::LOCALHOST.into(), a.port())
        }
        _ => listen,
    }
}

async fn echo_backend_accept_loop(listener: TcpListener) {
    let (stream, _) = listener.accept().await.expect("backend accept");
    let mut framed = Framed::new(stream, GdbCodec::new());
    while let Some(item) = framed.next().await {
        let item = item.expect("decode");
        match item {
            PacketOrAck::Packet(p) => {
                framed
                    .send(PacketOrAck::Packet(p))
                    .await
                    .expect("echo packet");
            }
            PacketOrAck::Ack => framed.send(PacketOrAck::Ack).await.expect("echo ack"),
            PacketOrAck::Nack => framed.send(PacketOrAck::Nack).await.expect("echo nack"),
        }
    }
}

async fn setup_proxy_to_backend(backend_port: u16) -> (SocketAddr, JoinHandle<()>) {
    let proxy_cfg = ProxyConfig {
        listen_port: 0,
        target_host: "127.0.0.1".to_string(),
        target_port: backend_port,
        enable_acks: true,
        timeout_secs: 5,
    };

    let mut server = ProxyServer::new(proxy_cfg, RecordingConfig::default(), None)
        .await
        .expect("proxy bind");
    let proxy_listen = server.local_addr().expect("proxy addr");

    let run = tokio::spawn(async move {
        let _ = server.run().await;
    });

    (proxy_listen, run)
}

#[tokio::test]
async fn proxy_forwards_rsp_packet_to_backend_and_back() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));

    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    let pkt = Packet::new(b"qEcho".to_vec());
    client
        .send(PacketOrAck::Packet(pkt))
        .await
        .expect("send packet");

    let got = client
        .next()
        .await
        .expect("stream ended")
        .expect("decode ok");
    match got {
        PacketOrAck::Packet(p) => assert_eq!(p.data.as_slice(), b"qEcho"),
        other => panic!("expected packet, got {:?}", other),
    }

    drop(client);
    backend_task.abort();
    run.abort();
}

/// Feature negotiation shape (`qSupported:…`) — must round-trip unchanged (Phase B transparency).
#[tokio::test]
async fn proxy_round_trips_qsupported_style_negotiation() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));
    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    let payload = b"qSupported:multiprocess+;xmlRegisters=i386;swbreak+;hwbreak+".as_slice();
    let pkt = Packet::new(payload.to_vec());
    client.send(PacketOrAck::Packet(pkt)).await.expect("send");

    let got = client
        .next()
        .await
        .expect("stream ended")
        .expect("decode ok");
    match got {
        PacketOrAck::Packet(p) => assert_eq!(p.data.as_slice(), payload),
        other => panic!("expected packet, got {:?}", other),
    }

    drop(client);
    backend_task.abort();
    run.abort();
}

/// GDB "last signal" query: `$?#3f` — exercises checksum path used by real sessions.
#[tokio::test]
async fn proxy_round_trips_last_signal_query_packet() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));
    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    let pkt = Packet::new(b"?".to_vec());
    client.send(PacketOrAck::Packet(pkt)).await.expect("send");

    let got = client
        .next()
        .await
        .expect("stream ended")
        .expect("decode ok");
    match got {
        PacketOrAck::Packet(p) => assert_eq!(p.data.as_slice(), b"?"),
        other => panic!("expected packet, got {:?}", other),
    }

    drop(client);
    backend_task.abort();
    run.abort();
}

#[tokio::test]
async fn proxy_round_trips_ack_and_nack() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));
    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    client.send(PacketOrAck::Ack).await.expect("ack");
    match client.next().await.expect("stream").expect("decode") {
        PacketOrAck::Ack => {}
        other => panic!("expected Ack, got {:?}", other),
    }

    client.send(PacketOrAck::Nack).await.expect("nack");
    match client.next().await.expect("stream").expect("decode") {
        PacketOrAck::Nack => {}
        other => panic!("expected Nack, got {:?}", other),
    }

    drop(client);
    backend_task.abort();
    run.abort();
}

#[tokio::test]
async fn proxy_round_trips_two_packets_sequentially() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));
    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    for payload in [b"qSupported".as_slice(), b"vCont?".as_slice()] {
        let pkt = Packet::new(payload.to_vec());
        client.send(PacketOrAck::Packet(pkt)).await.expect("send");
        let got = client.next().await.expect("stream").expect("decode");
        match got {
            PacketOrAck::Packet(p) => assert_eq!(p.data.as_slice(), payload),
            other => panic!("expected packet, got {:?}", other),
        }
    }

    drop(client);
    backend_task.abort();
    run.abort();
}

/// Larger payload to exercise framing through TCP (Phase A proxy hardening).
#[tokio::test]
async fn proxy_round_trips_medium_payload_packet() {
    let backend = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind backend");
    let backend_port = backend.local_addr().expect("backend addr").port();

    let backend_task = tokio::spawn(echo_backend_accept_loop(backend));
    let (proxy_listen, run) = setup_proxy_to_backend(backend_port).await;

    let client = tokio::net::TcpStream::connect(connect_addr(proxy_listen))
        .await
        .expect("connect to proxy");
    let mut client = Framed::new(client, GdbCodec::new());

    let payload: Vec<u8> = (0u8..=240).map(|i| b'0' + (i % 10)).collect();
    let pkt = Packet::new(payload.clone());
    client.send(PacketOrAck::Packet(pkt)).await.expect("send");

    let got = client
        .next()
        .await
        .expect("stream ended")
        .expect("decode ok");
    match got {
        PacketOrAck::Packet(p) => assert_eq!(p.data, payload),
        other => panic!("expected packet, got {:?}", other),
    }

    drop(client);
    backend_task.abort();
    run.abort();
}
