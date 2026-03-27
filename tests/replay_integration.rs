//! Integration: mock replay backend + proxy + client (issue #10).

use futures::{SinkExt, StreamExt};
use rsgdb::config::{ProxyConfig, RecordingConfig};
use rsgdb::protocol::codec::{GdbCodec, PacketOrAck};
use rsgdb::protocol::Packet;
use rsgdb::proxy::ProxyServer;
use rsgdb::recorder::{RecordDirection, RecordEventV1};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

#[tokio::test]
async fn replay_mock_backend_round_trip_through_proxy() {
    let events = vec![
        RecordEventV1::from_rsp(
            RecordDirection::ClientToBackend,
            &PacketOrAck::Packet(Packet::new(b"qSupported".to_vec())),
        ),
        RecordEventV1::from_rsp(
            RecordDirection::BackendToClient,
            &PacketOrAck::Packet(Packet::new(b"OK".to_vec())),
        ),
    ];

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
    let backend_port = listener.local_addr().expect("addr").port();

    tokio::spawn(async move {
        let (sock, _) = listener.accept().await.expect("mock accept");
        rsgdb::replay::run_mock_backend(sock, events)
            .await
            .expect("mock backend");
    });

    let mut server = ProxyServer::new(
        ProxyConfig {
            listen_port: 0,
            target_host: "127.0.0.1".into(),
            target_port: backend_port,
            enable_acks: true,
            timeout_secs: 30,
        },
        RecordingConfig::default(),
        None,
    )
    .await
    .expect("proxy");

    let proxy_addr = server.local_addr().expect("proxy addr");
    tokio::spawn(async move {
        let _ = server.run().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = TcpStream::connect(proxy_addr)
        .await
        .expect("connect to proxy");
    let mut framed = Framed::new(client, GdbCodec::new());

    framed
        .send(PacketOrAck::Packet(Packet::new(b"qSupported".to_vec())))
        .await
        .expect("send qSupported");

    let got = framed
        .next()
        .await
        .transpose()
        .expect("decode")
        .expect("one reply");
    match got {
        PacketOrAck::Packet(p) => assert_eq!(p.data, b"OK"),
        other => panic!("expected OK packet, got {other:?}"),
    }
}
