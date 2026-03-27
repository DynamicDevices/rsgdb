//! Backends module
//!
//! Debug probe backend implementations (OpenOCD, probe-rs, pyOCD).

/// Backend trait for debug probes
pub trait Backend {
    /// Connect to the target
    fn connect(&mut self) -> anyhow::Result<()>;

    /// Disconnect from the target
    fn disconnect(&mut self) -> anyhow::Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;
}

// Made with Bob
