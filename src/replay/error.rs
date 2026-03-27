//! Errors for session replay.

use crate::protocol::ProtocolError;

/// Replay / mock-backend failures.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid session header: {0}")]
    InvalidHeader(String),

    #[error("invalid record event: {0}")]
    InvalidEvent(String),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error("replay mismatch at step {step}: expected {expected}, got {got}")]
    Mismatch {
        step: usize,
        expected: String,
        got: String,
    },

    #[error("replay ended before mock backend finished (step {step})")]
    UnexpectedEof { step: usize },
}
