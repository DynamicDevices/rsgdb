//! Error types for rsgdb

use thiserror::Error;

use crate::protocol::{CommandError, ProtocolError};

/// Main error type for rsgdb
#[derive(Error, Debug)]
pub enum RsgdbError {
    /// Protocol-related errors
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Command parsing errors
    #[error("Command error: {0}")]
    Command(#[from] CommandError),

    /// Backend communication errors
    #[error("Backend error: {0}")]
    Backend(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// JSON serialization / deserialization
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// State errors
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Not implemented
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// File not found
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    /// Parse error
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    /// Validation error
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),

    /// Missing required field
    #[error("Missing required configuration field: {0}")]
    MissingField(String),

    /// Invalid value
    #[error("Invalid configuration value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
}

/// Result type alias for rsgdb operations
pub type Result<T> = std::result::Result<T, RsgdbError>;

/// Result type alias for configuration operations
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
