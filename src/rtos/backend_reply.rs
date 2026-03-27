//! Parse **backend → GDB** RSP payloads for read-only logging (Phase B). Does not modify packets.

/// Summarize thread-related **reply** payloads (not `T` stop replies — use [`super::summarize_stop_reply`]).
pub fn summarize_backend_thread_payload(data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    // Stop replies are handled separately (may contain binary).
    if data.first() == Some(&b'T') {
        return None;
    }

    let s = std::str::from_utf8(data).ok()?;

    if s == "l" {
        return Some("thread list: end (l)".to_string());
    }

    if let Some(rest) = s.strip_prefix('m') {
        if rest.is_empty() {
            return Some("thread list: m (empty)".to_string());
        }
        let n = rest.split(',').filter(|seg| !seg.is_empty()).count();
        return Some(format!("thread list: {n} id(s)"));
    }

    if let Some(rest) = s.strip_prefix("QC") {
        return Some(format!("current thread (qC reply): QC{rest}"));
    }

    // qThreadExtraInfo reply: hex-encoded UTF-8 name (ASCII hex digits only).
    if data.len() >= 2 && data.len() % 2 == 0 && data.iter().all(|b| b.is_ascii_hexdigit()) {
        if let Ok(bytes) = hex::decode(data) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.len() <= 512 {
                    return Some(format!("thread extra info (decoded): {text:?}"));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_list_end() {
        assert_eq!(
            summarize_backend_thread_payload(b"l").as_deref(),
            Some("thread list: end (l)")
        );
    }

    #[test]
    fn thread_list_ids() {
        let s = summarize_backend_thread_payload(b"m1,2,a").expect("summary");
        assert!(s.contains("3 id"), "{s}");
    }

    #[test]
    fn current_thread_qc() {
        let s = summarize_backend_thread_payload(b"QC1").expect("summary");
        assert!(s.contains("QC1"), "{s}");
    }

    #[test]
    fn thread_name_hex() {
        // "main" as UTF-8 hex: 6d61696e
        let s = summarize_backend_thread_payload(b"6d61696e").expect("summary");
        assert!(s.contains("main"), "{s}");
    }

    #[test]
    fn t_packet_not_handled_here() {
        assert!(summarize_backend_thread_payload(b"T05").is_none());
    }
}
