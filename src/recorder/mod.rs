//! Session recording (ordered RSP trace as **rsgdb-record** JSON Lines).

mod jsonl;

pub use jsonl::{
    RecordDirection, RecordEventV1, RecordHeaderV1, RecordKind, SessionRecorder, FORMAT_NAME,
    FORMAT_VERSION,
};
