//! State module
//!
//! State tracking and inspection for debugging sessions.

/// Represents the target state
#[derive(Debug, Clone, PartialEq)]
pub enum TargetState {
    /// Target is running
    Running,
    /// Target is stopped
    Stopped,
    /// Target is halted at a breakpoint
    Halted,
    /// Target state is unknown
    Unknown,
}

// Made with Bob
