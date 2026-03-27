//! Phase A — RSP framing regression: codec edge cases without TCP (no gdb binary).
//!
//! Complements `tests/proxy_integration.rs` (TCP + proxy). Run alone:
//!   cargo test --all-features --test rsp_codec_matrix

use bytes::BytesMut;
use rsgdb::protocol::codec::{GdbCodec, PacketOrAck};
use rsgdb::protocol::Packet;
use rsgdb::protocol::ProtocolError;
use tokio_util::codec::Decoder;

#[test]
fn decode_nack() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"-"[..]);
    let r = codec.decode(&mut buf).unwrap();
    assert!(matches!(r, Some(PacketOrAck::Nack)));
    assert!(buf.is_empty());
}

#[test]
fn decode_leading_junk_before_dollar() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"noise$OK#9a"[..]);
    let r = codec.decode(&mut buf).unwrap();
    match r {
        Some(PacketOrAck::Packet(p)) => {
            assert_eq!(p.data, b"OK");
        }
        other => panic!("expected packet, got {:?}", other),
    }
    assert!(buf.is_empty());
}

#[test]
fn decode_packet_split_across_two_feeds() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"$qSuppor"[..]);
    assert!(codec.decode(&mut buf).unwrap().is_none());
    assert!(!buf.is_empty());

    buf.extend_from_slice(b"ted#37");
    let r = codec.decode(&mut buf).unwrap();
    match r {
        Some(PacketOrAck::Packet(p)) => assert_eq!(p.data, b"qSupported"),
        other => panic!("expected packet, got {:?}", other),
    }
    assert!(buf.is_empty());
}

#[test]
fn decode_ack_then_packet_in_one_buffer() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"+$OK#9a"[..]);

    assert!(matches!(
        codec.decode(&mut buf).unwrap(),
        Some(PacketOrAck::Ack)
    ));
    match codec.decode(&mut buf).unwrap() {
        Some(PacketOrAck::Packet(p)) => assert_eq!(p.data, b"OK"),
        other => panic!("expected second item packet, got {:?}", other),
    }
    assert!(buf.is_empty());
}

#[test]
fn decode_invalid_checksum_errors() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"$qSupported#00"[..]);
    let err = codec.decode(&mut buf).unwrap_err();
    assert!(matches!(err, ProtocolError::ChecksumMismatch { .. }));
}

#[test]
fn decode_long_payload_round_trip() {
    let data: Vec<u8> = (0u8..=200).map(|i| b'a' + (i % 26)).collect();
    let packet = Packet::new(data.clone());
    let wire = packet.to_bytes();

    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&wire[..]);
    match codec.decode(&mut buf).unwrap() {
        Some(PacketOrAck::Packet(p)) => assert_eq!(p.data, data),
        other => panic!("expected packet, got {:?}", other),
    }
}

#[test]
fn decode_empty_buffer_returns_none_and_stays_empty() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::new();
    assert!(codec.decode(&mut buf).unwrap().is_none());
    assert!(buf.is_empty());
}

#[test]
fn decode_no_rsp_marker_clears_buffer() {
    let mut codec = GdbCodec::new();
    let mut buf = BytesMut::from(&b"no_marker_here"[..]);
    assert!(codec.decode(&mut buf).unwrap().is_none());
    assert!(buf.is_empty());
}
