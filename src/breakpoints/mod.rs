//! Breakpoints module
//!
//! Advanced breakpoint management with support for named, conditional,
//! and grouped breakpoints.

/// Breakpoint type
#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointType {
    /// Software breakpoint
    Software,
    /// Hardware breakpoint
    Hardware,
    /// Watchpoint (data breakpoint)
    Watchpoint,
}

/// Represents a breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique identifier
    pub id: u32,

    /// Optional name
    pub name: Option<String>,

    /// Address
    pub address: u64,

    /// Breakpoint type
    pub bp_type: BreakpointType,

    /// Whether the breakpoint is enabled
    pub enabled: bool,

    /// Optional condition
    pub condition: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_creation() {
        let bp = Breakpoint {
            id: 1,
            name: Some("main".to_string()),
            address: 0x8000,
            bp_type: BreakpointType::Hardware,
            enabled: true,
            condition: None,
        };
        assert_eq!(bp.id, 1);
        assert_eq!(bp.address, 0x8000);
    }
}
