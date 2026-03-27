//! JSON Lines session recording — format **rsgdb-record v1** (`.jsonl`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::config::RecordingConfig;
use crate::error::RsgdbError;
use crate::protocol::codec::PacketOrAck;
use tracing::{info, warn};

pub const FORMAT_NAME: &str = "rsgdb-record";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordHeaderV1 {
    pub format: String,
    pub version: u32,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordDirection {
    ClientToBackend,
    BackendToClient,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Packet,
    Ack,
    Nack,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordEventV1 {
    pub ts: DateTime<Utc>,
    pub direction: RecordDirection,
    pub kind: RecordKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_len: Option<usize>,
}

impl RecordEventV1 {
    pub fn from_rsp(direction: RecordDirection, item: &PacketOrAck) -> Self {
        let ts = Utc::now();
        match item {
            PacketOrAck::Packet(p) => Self {
                ts,
                direction,
                kind: RecordKind::Packet,
                payload_hex: Some(hex::encode(&p.data)),
                payload_len: Some(p.data.len()),
            },
            PacketOrAck::Ack => Self {
                ts,
                direction,
                kind: RecordKind::Ack,
                payload_hex: None,
                payload_len: None,
            },
            PacketOrAck::Nack => Self {
                ts,
                direction,
                kind: RecordKind::Nack,
                payload_hex: None,
                payload_len: None,
            },
        }
    }
}

/// Append-only JSONL writer with optional byte cap (`max_bytes == 0` = unlimited).
pub struct SessionRecorder {
    path: PathBuf,
    writer: BufWriter<tokio::fs::File>,
    bytes_written: u64,
    max_bytes: u64,
    stopped: bool,
}

impl SessionRecorder {
    pub async fn create(cfg: &RecordingConfig) -> Result<Self, RsgdbError> {
        if cfg.compress {
            warn!(
                "recording.compress=true is not implemented for format v1; writing uncompressed JSONL"
            );
        }

        tokio::fs::create_dir_all(&cfg.output_dir).await?;

        let session_id = format!(
            "{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            std::process::id()
        );
        let filename = format!("{}.jsonl", session_id);
        let path = Path::new(&cfg.output_dir).join(&filename);

        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await?;

        let writer = BufWriter::new(file);

        let header = RecordHeaderV1 {
            format: FORMAT_NAME.to_string(),
            version: FORMAT_VERSION,
            session_id,
            started_at: Utc::now(),
        };

        let max_bytes = cfg.max_size_mb.saturating_mul(1024 * 1024);

        let mut rec = Self {
            path,
            writer,
            bytes_written: 0,
            max_bytes,
            stopped: false,
        };

        let line = serde_json::to_string(&header)?;
        rec.write_line(&line).await?;

        info!(
            path = %rec.path.display(),
            max_bytes = rec.max_bytes,
            "RSP session recording started (format v{})",
            FORMAT_VERSION
        );

        Ok(rec)
    }

    async fn write_line(&mut self, line: &str) -> Result<(), RsgdbError> {
        if self.stopped {
            return Ok(());
        }
        let need = line.len() as u64 + 1;
        if self.max_bytes > 0 && self.bytes_written + need > self.max_bytes {
            warn!(
                path = %self.path.display(),
                limit = self.max_bytes,
                "recording size limit reached; stopping further writes"
            );
            self.stopped = true;
            return Ok(());
        }
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.bytes_written += need;
        Ok(())
    }

    pub async fn record(&mut self, ev: &RecordEventV1) -> Result<(), RsgdbError> {
        let line = serde_json::to_string(ev)?;
        self.write_line(&line).await
    }

    pub async fn flush(&mut self) -> Result<(), RsgdbError> {
        self.writer.flush().await.map_err(RsgdbError::Io)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Packet;
    use tempfile::tempdir;

    #[tokio::test]
    async fn jsonl_header_and_packet_round_trip_parse() {
        let dir = tempdir().unwrap();
        let cfg = RecordingConfig {
            enabled: true,
            output_dir: dir.path().to_string_lossy().into_owned(),
            max_size_mb: 1,
            compress: false,
        };

        let mut rec = SessionRecorder::create(&cfg).await.expect("recorder");
        let pkt = Packet::new(b"vCont?".to_vec());
        let ev =
            RecordEventV1::from_rsp(RecordDirection::ClientToBackend, &PacketOrAck::Packet(pkt));
        rec.record(&ev).await.expect("record");
        rec.flush().await.expect("flush");

        let text = std::fs::read_to_string(rec.path()).expect("read");
        let mut lines = text.lines();
        let h: RecordHeaderV1 = serde_json::from_str(lines.next().unwrap()).expect("header");
        assert_eq!(h.format, FORMAT_NAME);
        assert_eq!(h.version, FORMAT_VERSION);

        let e: RecordEventV1 = serde_json::from_str(lines.next().unwrap()).expect("event");
        assert!(matches!(e.kind, RecordKind::Packet));
        assert_eq!(e.payload_len, Some(6));
    }
}
