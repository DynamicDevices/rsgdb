//! Load **rsgdb-record v1** JSONL files for replay.

use std::path::Path;

use crate::recorder::{RecordEventV1, RecordHeaderV1, FORMAT_NAME, FORMAT_VERSION};

use super::ReplayError;

/// Parsed recording: header + ordered events (excluding the header line).
#[derive(Debug, Clone)]
pub struct LoadedSession {
    pub header: RecordHeaderV1,
    pub events: Vec<RecordEventV1>,
}

/// Read a `.jsonl` session file; validates format name/version on line 1.
pub fn load_session(path: &Path) -> Result<LoadedSession, ReplayError> {
    let text = std::fs::read_to_string(path)?;
    load_session_str(&text)
}

fn load_session_str(text: &str) -> Result<LoadedSession, ReplayError> {
    let mut lines = text.lines();
    let first = lines
        .next()
        .ok_or_else(|| ReplayError::InvalidHeader("empty file".into()))?;
    let header: RecordHeaderV1 = serde_json::from_str(first)?;
    if header.format != FORMAT_NAME {
        return Err(ReplayError::InvalidHeader(format!(
            "expected format {:?}, got {:?}",
            FORMAT_NAME, header.format
        )));
    }
    if header.version != FORMAT_VERSION {
        return Err(ReplayError::InvalidHeader(format!(
            "expected version {}, got {}",
            FORMAT_VERSION, header.version
        )));
    }

    let mut events = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let ev: RecordEventV1 = serde_json::from_str(line)?;
        events.push(ev);
    }

    Ok(LoadedSession { header, events })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::{RecordDirection, RecordKind};
    use chrono::Utc;

    #[test]
    fn load_round_trip() {
        let h = RecordHeaderV1 {
            format: FORMAT_NAME.to_string(),
            version: FORMAT_VERSION,
            session_id: "t".into(),
            started_at: Utc::now(),
        };
        let ev = RecordEventV1 {
            ts: Utc::now(),
            direction: RecordDirection::ClientToBackend,
            kind: RecordKind::Packet,
            payload_hex: Some("71737570706f72746564".into()), // "qsupported"
            payload_len: Some(10),
        };
        let mut s = serde_json::to_string(&h).unwrap();
        s.push('\n');
        s.push_str(&serde_json::to_string(&ev).unwrap());

        let loaded = load_session_str(&s).expect("load");
        assert_eq!(loaded.events.len(), 1);
        assert!(matches!(loaded.events[0].kind, RecordKind::Packet));
    }
}
