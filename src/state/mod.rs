//! State module
//!
//! State tracking and inspection for debugging sessions.
//!
//! **Status — scaffold:** not wired into the proxy; placeholder enum for future session/target
//! state. Safe to ignore unless you are extending this API.

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
