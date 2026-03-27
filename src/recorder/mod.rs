//! Recorder module
//!
//! Session recording and replay functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a recorded debugging session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    
    /// Start time
    pub start_time: DateTime<Utc>,
    
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    
    /// Recorded events
    pub events: Vec<SessionEvent>,
}

/// Represents an event in a debugging session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Event type
    pub event_type: String,
    
    /// Event data
    pub data: Vec<u8>,
}

// Made with Bob
