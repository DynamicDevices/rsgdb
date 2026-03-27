//! Protocol module
//!
//! GDB Remote Serial Protocol (RSP) implementation.

pub mod commands;
pub mod codec;

use thiserror::Error;

pub use commands::{GdbCommand, QueryCommand, BreakpointType, CommandError};
pub use codec::{GdbCodec, PacketOrAck};

/// Protocol errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid packet format")]
    InvalidFormat,
    
    #[error("Checksum mismatch: expected {expected:02x}, got {actual:02x}")]
    ChecksumMismatch { expected: u8, actual: u8 },
    
    #[error("Incomplete packet")]
    IncompletePacket,
    
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Represents a GDB RSP packet
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    /// The packet data (without $ prefix, # suffix, or checksum)
    pub data: Vec<u8>,
    
    /// The checksum
    pub checksum: u8,
}

impl Packet {
    /// Create a new packet with the given data
    pub fn new(data: Vec<u8>) -> Self {
        let checksum = Self::calculate_checksum(&data);
        Self { data, checksum }
    }
    
    /// Calculate the checksum for the given data
    pub fn calculate_checksum(data: &[u8]) -> u8 {
        data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
    }
    
    /// Parse a packet from a buffer
    /// Format: $<data>#<checksum>
    pub fn parse(buffer: &[u8]) -> Result<Self, ProtocolError> {
        // Check minimum length: $#XX (4 bytes)
        if buffer.len() < 4 {
            return Err(ProtocolError::IncompletePacket);
        }
        
        // Check for $ prefix
        if buffer[0] != b'$' {
            return Err(ProtocolError::InvalidFormat);
        }
        
        // Find the # separator
        let hash_pos = buffer.iter().position(|&b| b == b'#')
            .ok_or(ProtocolError::InvalidFormat)?;
        
        // Check we have room for checksum
        if hash_pos + 3 > buffer.len() {
            return Err(ProtocolError::IncompletePacket);
        }
        
        // Extract data (between $ and #)
        let data = buffer[1..hash_pos].to_vec();
        
        // Parse checksum (2 hex digits after #)
        let checksum_str = std::str::from_utf8(&buffer[hash_pos + 1..hash_pos + 3])
            .map_err(|_| ProtocolError::InvalidFormat)?;
        let checksum = u8::from_str_radix(checksum_str, 16)
            .map_err(|_| ProtocolError::InvalidFormat)?;
        
        // Verify checksum
        let calculated = Self::calculate_checksum(&data);
        if calculated != checksum {
            return Err(ProtocolError::ChecksumMismatch {
                expected: calculated,
                actual: checksum,
            });
        }
        
        Ok(Self { data, checksum })
    }
    
    /// Serialize the packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.data.len() + 4);
        result.push(b'$');
        result.extend_from_slice(&self.data);
        result.push(b'#');
        result.extend_from_slice(&format!("{:02x}", self.checksum).as_bytes());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_calculation() {
        let data = b"qSupported";
        let checksum = Packet::calculate_checksum(data);
        // Sum of ASCII values mod 256: q(113)+S(83)+u(117)+p(112)+p(112)+o(111)+r(114)+t(116)+e(101)+d(100) = 1079 % 256 = 55 (0x37)
        assert_eq!(checksum, 0x37);
    }

    #[test]
    fn test_packet_parse_valid() {
        let buffer = b"$qSupported#37";
        let packet = Packet::parse(buffer).unwrap();
        assert_eq!(packet.data, b"qSupported");
        assert_eq!(packet.checksum, 0x37);
    }

    #[test]
    fn test_packet_parse_invalid_checksum() {
        let buffer = b"$qSupported#00";
        let result = Packet::parse(buffer);
        assert!(matches!(result, Err(ProtocolError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_packet_to_bytes() {
        let packet = Packet::new(b"qSupported".to_vec());
        let bytes = packet.to_bytes();
        assert_eq!(bytes, b"$qSupported#37");
    }
}

// Made with Bob
