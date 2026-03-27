//! Tokio codec for GDB RSP packet streaming

use bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::{Packet, ProtocolError};

/// Codec for encoding/decoding GDB RSP packets from a byte stream
#[derive(Debug, Default)]
pub struct GdbCodec;

impl GdbCodec {
    /// Create a new GDB codec
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Find the start of a packet in the buffer
    fn find_packet_start(buf: &[u8]) -> Option<usize> {
        buf.iter().position(|&b| b == b'$' || b == b'+' || b == b'-')
    }
    
    /// Find the end of a packet in the buffer
    fn find_packet_end(buf: &[u8], start: usize) -> Option<usize> {
        if start >= buf.len() {
            return None;
        }
        
        // Handle ACK/NACK (single byte)
        if buf[start] == b'+' || buf[start] == b'-' {
            return Some(start + 1);
        }
        
        // Handle regular packet: $...#XX
        if buf[start] != b'$' {
            return None;
        }
        
        // Find the # separator
        let hash_pos = buf[start..].iter().position(|&b| b == b'#')?;
        let hash_pos = start + hash_pos;
        
        // Check if we have the full checksum (2 hex digits after #)
        if hash_pos + 3 <= buf.len() {
            Some(hash_pos + 3)
        } else {
            None
        }
    }
}

impl Decoder for GdbCodec {
    type Item = PacketOrAck;
    type Error = ProtocolError;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Find packet start
        let start = match Self::find_packet_start(src) {
            Some(pos) => pos,
            None => {
                // No packet start found, discard everything
                src.clear();
                return Ok(None);
            }
        };
        
        // Discard any data before the packet start
        if start > 0 {
            src.advance(start);
        }
        
        // Find packet end
        let end = match Self::find_packet_end(src, 0) {
            Some(pos) => pos,
            None => {
                // Incomplete packet, need more data
                // Reserve space for potential large packet
                src.reserve(1024);
                return Ok(None);
            }
        };
        
        // Extract the packet data
        let packet_data = src[..end].to_vec();
        src.advance(end);
        
        // Parse based on first byte
        match packet_data[0] {
            b'+' => Ok(Some(PacketOrAck::Ack)),
            b'-' => Ok(Some(PacketOrAck::Nack)),
            b'$' => {
                let packet = Packet::parse(&packet_data)?;
                Ok(Some(PacketOrAck::Packet(packet)))
            }
            _ => Err(ProtocolError::InvalidFormat),
        }
    }
}

impl Encoder<PacketOrAck> for GdbCodec {
    type Error = ProtocolError;
    
    fn encode(&mut self, item: PacketOrAck, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            PacketOrAck::Ack => {
                dst.extend_from_slice(b"+");
            }
            PacketOrAck::Nack => {
                dst.extend_from_slice(b"-");
            }
            PacketOrAck::Packet(packet) => {
                let bytes = packet.to_bytes();
                dst.extend_from_slice(&bytes);
            }
        }
        Ok(())
    }
}

/// Represents either a packet or an ACK/NACK
#[derive(Debug, Clone, PartialEq)]
pub enum PacketOrAck {
    /// A regular GDB packet
    Packet(Packet),
    /// Acknowledgment (+)
    Ack,
    /// Negative acknowledgment (-)
    Nack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_packet() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::from(&b"$qSupported#37"[..]);
        
        let result = codec.decode(&mut buf).unwrap();
        assert!(matches!(result, Some(PacketOrAck::Packet(_))));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_decode_ack() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::from(&b"+"[..]);
        
        let result = codec.decode(&mut buf).unwrap();
        assert!(matches!(result, Some(PacketOrAck::Ack)));
    }

    #[test]
    fn test_decode_incomplete_packet() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::from(&b"$qSupported#3"[..]);
        
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
        assert_eq!(buf.len(), 13); // Data still in buffer
    }

    #[test]
    fn test_decode_multiple_packets() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::from(&b"+$qSupported#37"[..]);
        
        // First decode should get the ACK
        let result = codec.decode(&mut buf).unwrap();
        assert!(matches!(result, Some(PacketOrAck::Ack)));
        
        // Second decode should get the packet
        let result = codec.decode(&mut buf).unwrap();
        assert!(matches!(result, Some(PacketOrAck::Packet(_))));
    }

    #[test]
    fn test_encode_packet() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::new();
        let packet = Packet::new(b"OK".to_vec());
        
        codec.encode(PacketOrAck::Packet(packet), &mut buf).unwrap();
        assert_eq!(&buf[..], b"$OK#9a");
    }

    #[test]
    fn test_encode_ack() {
        let mut codec = GdbCodec::new();
        let mut buf = BytesMut::new();
        
        codec.encode(PacketOrAck::Ack, &mut buf).unwrap();
        assert_eq!(&buf[..], b"+");
    }
}

// Made with Bob
