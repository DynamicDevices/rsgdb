//! GDB RSP command definitions and parsing

use thiserror::Error;

/// GDB command parsing errors
#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Unknown command: {0}")]
    Unknown(String),

    #[error("Invalid command format: {0}")]
    InvalidFormat(String),

    #[error("Invalid hex value: {0}")]
    InvalidHex(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
}

/// Represents a parsed GDB command
#[derive(Debug, Clone, PartialEq)]
pub enum GdbCommand {
    /// Query command (q*)
    Query(QueryCommand),

    /// Set command (Q*)
    Set(String, String),

    /// Read registers (g)
    ReadRegisters,

    /// Write registers (G)
    WriteRegisters(Vec<u8>),

    /// Read memory (m address,length)
    ReadMemory { addr: u64, len: usize },

    /// Write memory (M address,length:XX...)
    WriteMemory { addr: u64, data: Vec<u8> },

    /// Continue execution (c \[address\])
    Continue { addr: Option<u64> },

    /// Step execution (s \[address\])
    Step { addr: Option<u64> },

    /// Insert breakpoint (Z type,address,kind)
    InsertBreakpoint {
        bp_type: BreakpointType,
        addr: u64,
        kind: u32,
    },

    /// Remove breakpoint (z type,address,kind)
    RemoveBreakpoint {
        bp_type: BreakpointType,
        addr: u64,
        kind: u32,
    },

    /// Kill request (k)
    Kill,

    /// Detach (D)
    Detach,

    /// Hg / Hc — set thread for subsequent operations (`Hg` = general / registers, `Hc` = continue)
    SetThread {
        /// `true` if `Hc…`, `false` if `Hg…`
        for_continue: bool,
        thread_id: u64,
    },

    /// vCont - Continue with actions
    VCont(VContAction),

    /// Unknown/unsupported command
    Unsupported(String),
}

/// Query commands (q*)
#[derive(Debug, Clone, PartialEq)]
pub enum QueryCommand {
    /// qSupported - Feature negotiation
    Supported(Vec<String>),

    /// qAttached - Query if attached to existing process
    Attached,

    /// qC - Current thread ID
    CurrentThread,

    /// qfThreadInfo - First thread info
    FirstThreadInfo,

    /// qsThreadInfo - Subsequent thread info
    SubsequentThreadInfo,

    /// qThreadExtraInfo — thread name / extra info for a thread id (hex-encoded id after `:`)
    ThreadExtraInfo(String),

    /// qOffsets - Section offsets
    Offsets,

    /// qSymbol - Symbol lookup
    Symbol(Option<String>),

    /// Other query
    Other(String),
}

/// Breakpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointType {
    /// Software breakpoint
    Software = 0,
    /// Hardware breakpoint
    Hardware = 1,
    /// Write watchpoint
    WriteWatchpoint = 2,
    /// Read watchpoint
    ReadWatchpoint = 3,
    /// Access watchpoint
    AccessWatchpoint = 4,
}

impl TryFrom<u8> for BreakpointType {
    type Error = CommandError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BreakpointType::Software),
            1 => Ok(BreakpointType::Hardware),
            2 => Ok(BreakpointType::WriteWatchpoint),
            3 => Ok(BreakpointType::ReadWatchpoint),
            4 => Ok(BreakpointType::AccessWatchpoint),
            _ => Err(CommandError::InvalidFormat(format!(
                "Invalid breakpoint type: {}",
                value
            ))),
        }
    }
}

/// vCont action
#[derive(Debug, Clone, PartialEq)]
pub enum VContAction {
    /// Continue
    Continue,
    /// Continue with signal
    ContinueWithSignal(u8),
    /// Step
    Step,
    /// Step with signal
    StepWithSignal(u8),
    /// Stop
    Stop,
}

impl GdbCommand {
    /// Parse a GDB command from packet data
    pub fn parse(data: &[u8]) -> Result<Self, CommandError> {
        if data.is_empty() {
            return Err(CommandError::InvalidFormat("Empty command".to_string()));
        }

        let cmd_str = std::str::from_utf8(data)
            .map_err(|_| CommandError::InvalidFormat("Invalid UTF-8".to_string()))?;

        match data[0] {
            b'q' => Self::parse_query(&cmd_str[1..]),
            b'Q' => Self::parse_set(&cmd_str[1..]),
            b'g' => Ok(GdbCommand::ReadRegisters),
            b'G' => Self::parse_write_registers(&cmd_str[1..]),
            b'm' => Self::parse_read_memory(&cmd_str[1..]),
            b'M' => Self::parse_write_memory(&cmd_str[1..]),
            b'c' => Self::parse_continue(&cmd_str[1..]),
            b's' => Self::parse_step(&cmd_str[1..]),
            b'Z' => Self::parse_insert_breakpoint(&cmd_str[1..]),
            b'z' => Self::parse_remove_breakpoint(&cmd_str[1..]),
            b'k' => Ok(GdbCommand::Kill),
            b'D' => Ok(GdbCommand::Detach),
            b'H' => Self::parse_set_thread(&cmd_str[1..]),
            b'v' => Self::parse_v_command(&cmd_str[1..]),
            _ => Ok(GdbCommand::Unsupported(cmd_str.to_string())),
        }
    }

    fn parse_set_thread(data: &str) -> Result<Self, CommandError> {
        let mut chars = data.chars();
        let mode = chars
            .next()
            .ok_or_else(|| CommandError::InvalidFormat("H: missing g/c".to_string()))?;
        let for_continue = match mode {
            'g' => false,
            'c' => true,
            _ => {
                return Err(CommandError::InvalidFormat(format!(
                    "H: expected g or c, got {mode:?}"
                )));
            }
        };
        let rest: String = chars.collect();
        if rest.is_empty() {
            return Err(CommandError::InvalidFormat(
                "H: missing thread id".to_string(),
            ));
        }
        let thread_id =
            u64::from_str_radix(&rest, 16).map_err(|_| CommandError::InvalidHex(rest.clone()))?;
        Ok(GdbCommand::SetThread {
            for_continue,
            thread_id,
        })
    }

    fn parse_query(data: &str) -> Result<Self, CommandError> {
        if data.starts_with("Supported") {
            let features = if data.len() > 9 && data.chars().nth(9) == Some(':') {
                data[10..].split(';').map(String::from).collect()
            } else {
                Vec::new()
            };
            Ok(GdbCommand::Query(QueryCommand::Supported(features)))
        } else if data == "Attached" {
            Ok(GdbCommand::Query(QueryCommand::Attached))
        } else if data == "C" {
            Ok(GdbCommand::Query(QueryCommand::CurrentThread))
        } else if data == "fThreadInfo" {
            Ok(GdbCommand::Query(QueryCommand::FirstThreadInfo))
        } else if data == "sThreadInfo" {
            Ok(GdbCommand::Query(QueryCommand::SubsequentThreadInfo))
        } else if let Some(id) = data.strip_prefix("ThreadExtraInfo:") {
            Ok(GdbCommand::Query(QueryCommand::ThreadExtraInfo(
                id.to_string(),
            )))
        } else if data == "Offsets" {
            Ok(GdbCommand::Query(QueryCommand::Offsets))
        } else if data.starts_with("Symbol") {
            let symbol = if data.len() > 7 {
                Some(data[7..].to_string())
            } else {
                None
            };
            Ok(GdbCommand::Query(QueryCommand::Symbol(symbol)))
        } else {
            Ok(GdbCommand::Query(QueryCommand::Other(data.to_string())))
        }
    }

    fn parse_set(data: &str) -> Result<Self, CommandError> {
        if let Some(pos) = data.find(':') {
            let key = data[..pos].to_string();
            let value = data[pos + 1..].to_string();
            Ok(GdbCommand::Set(key, value))
        } else {
            Err(CommandError::InvalidFormat(
                "Set command missing ':'".to_string(),
            ))
        }
    }

    fn parse_write_registers(data: &str) -> Result<Self, CommandError> {
        let bytes = hex::decode(data).map_err(|_| CommandError::InvalidHex(data.to_string()))?;
        Ok(GdbCommand::WriteRegisters(bytes))
    }

    fn parse_read_memory(data: &str) -> Result<Self, CommandError> {
        let parts: Vec<&str> = data.split(',').collect();
        if parts.len() != 2 {
            return Err(CommandError::InvalidFormat(
                "Expected addr,length".to_string(),
            ));
        }

        let addr = u64::from_str_radix(parts[0], 16)
            .map_err(|_| CommandError::InvalidHex(parts[0].to_string()))?;
        let len = usize::from_str_radix(parts[1], 16)
            .map_err(|_| CommandError::InvalidHex(parts[1].to_string()))?;

        Ok(GdbCommand::ReadMemory { addr, len })
    }

    fn parse_write_memory(data: &str) -> Result<Self, CommandError> {
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() != 2 {
            return Err(CommandError::InvalidFormat(
                "Expected addr,length:data".to_string(),
            ));
        }

        let addr_len: Vec<&str> = parts[0].split(',').collect();
        if addr_len.len() != 2 {
            return Err(CommandError::InvalidFormat(
                "Expected addr,length".to_string(),
            ));
        }

        let addr = u64::from_str_radix(addr_len[0], 16)
            .map_err(|_| CommandError::InvalidHex(addr_len[0].to_string()))?;
        let data =
            hex::decode(parts[1]).map_err(|_| CommandError::InvalidHex(parts[1].to_string()))?;

        Ok(GdbCommand::WriteMemory { addr, data })
    }

    fn parse_continue(data: &str) -> Result<Self, CommandError> {
        let addr = if data.is_empty() {
            None
        } else {
            Some(
                u64::from_str_radix(data, 16)
                    .map_err(|_| CommandError::InvalidHex(data.to_string()))?,
            )
        };
        Ok(GdbCommand::Continue { addr })
    }

    fn parse_step(data: &str) -> Result<Self, CommandError> {
        let addr = if data.is_empty() {
            None
        } else {
            Some(
                u64::from_str_radix(data, 16)
                    .map_err(|_| CommandError::InvalidHex(data.to_string()))?,
            )
        };
        Ok(GdbCommand::Step { addr })
    }

    fn parse_insert_breakpoint(data: &str) -> Result<Self, CommandError> {
        Self::parse_breakpoint_command(data, true)
    }

    fn parse_remove_breakpoint(data: &str) -> Result<Self, CommandError> {
        Self::parse_breakpoint_command(data, false)
    }

    fn parse_breakpoint_command(data: &str, insert: bool) -> Result<Self, CommandError> {
        let parts: Vec<&str> = data.split(',').collect();
        if parts.len() != 3 {
            return Err(CommandError::InvalidFormat(
                "Expected type,addr,kind".to_string(),
            ));
        }

        let bp_type = parts[0]
            .parse::<u8>()
            .map_err(|_| CommandError::InvalidFormat(parts[0].to_string()))?
            .try_into()?;
        let addr = u64::from_str_radix(parts[1], 16)
            .map_err(|_| CommandError::InvalidHex(parts[1].to_string()))?;
        let kind = u32::from_str_radix(parts[2], 16)
            .map_err(|_| CommandError::InvalidHex(parts[2].to_string()))?;

        if insert {
            Ok(GdbCommand::InsertBreakpoint {
                bp_type,
                addr,
                kind,
            })
        } else {
            Ok(GdbCommand::RemoveBreakpoint {
                bp_type,
                addr,
                kind,
            })
        }
    }

    fn parse_v_command(data: &str) -> Result<Self, CommandError> {
        if data.starts_with("Cont") {
            // Simplified vCont parsing
            Ok(GdbCommand::VCont(VContAction::Continue))
        } else {
            Ok(GdbCommand::Unsupported(format!("v{}", data)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_supported() {
        let cmd = GdbCommand::parse(b"qSupported:multiprocess+").unwrap();
        assert!(matches!(cmd, GdbCommand::Query(QueryCommand::Supported(_))));
    }

    #[test]
    fn test_parse_read_memory() {
        let cmd = GdbCommand::parse(b"m8000,100").unwrap();
        assert!(matches!(
            cmd,
            GdbCommand::ReadMemory {
                addr: 0x8000,
                len: 0x100
            }
        ));
    }

    #[test]
    fn test_parse_insert_breakpoint() {
        let cmd = GdbCommand::parse(b"Z0,8000,2").unwrap();
        assert!(matches!(
            cmd,
            GdbCommand::InsertBreakpoint {
                bp_type: BreakpointType::Software,
                addr: 0x8000,
                kind: 2
            }
        ));
    }

    #[test]
    fn test_parse_continue() {
        let cmd = GdbCommand::parse(b"c").unwrap();
        assert!(matches!(cmd, GdbCommand::Continue { addr: None }));
    }

    #[test]
    fn test_parse_set_thread_hc() {
        let cmd = GdbCommand::parse(b"Hc2").unwrap();
        assert!(matches!(
            cmd,
            GdbCommand::SetThread {
                for_continue: true,
                thread_id: 2
            }
        ));
    }

    #[test]
    fn test_parse_set_thread_hg_id() {
        let cmd = GdbCommand::parse(b"Hg1").unwrap();
        assert!(matches!(
            cmd,
            GdbCommand::SetThread {
                for_continue: false,
                thread_id: 1
            }
        ));
    }

    #[test]
    fn test_parse_thread_extra_info() {
        let cmd = GdbCommand::parse(b"qThreadExtraInfo:1").unwrap();
        match cmd {
            GdbCommand::Query(QueryCommand::ThreadExtraInfo(id)) => assert_eq!(id, "1"),
            _ => panic!("expected ThreadExtraInfo"),
        }
    }
}
