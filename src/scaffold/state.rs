//! Target/session state — scaffold for future tracking.
//!
//! **Status:** Not wired into the proxy. Safe to ignore unless extending this API.

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
