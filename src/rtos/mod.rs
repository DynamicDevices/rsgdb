//! RTOS-related RSP helpers (decode / log only — thread lists come from the debug stub, e.g. OpenOCD + Zephyr).
//!
//! **Zephyr** is the primary workflow we document: use a stub that exposes GDB thread extensions (OpenOCD
//! `zephyr` RTOS awareness or equivalent). The wire protocol is standard GDB RSP; FreeRTOS, ThreadX, etc.
//! use the same packet shapes when the stub implements them.

/// Summarize a stop-reply `T…` packet for logging (often includes `thread:…` from RTOS-aware stubs).
pub fn summarize_stop_reply(data: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(data).ok()?;
    if !s.starts_with('T') || s.len() < 2 {
        return None;
    }
    // Optional: "thread:hexid;" (GDB RSP thread id)
    if let Some(idx) = s.find("thread:") {
        let rest = &s[idx + "thread:".len()..];
        let end = rest.find(';').unwrap_or(rest.len());
        let tid = rest[..end].trim();
        if !tid.is_empty() {
            return Some(format!("T stop reply, thread={tid}"));
        }
    }
    Some("T stop reply".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_reply_with_thread() {
        let s = summarize_stop_reply(b"T05thread:1;").expect("summary");
        assert!(s.contains("thread=1"), "{s}");
    }

    #[test]
    fn stop_reply_minimal() {
        let s = summarize_stop_reply(b"T05").expect("summary");
        assert_eq!(s, "T stop reply");
    }
}
