//! Replay **rsgdb-record v1** JSONL sessions through a mock TCP backend (issue #10).
//!
//! Use [`run_mock_backend`] after accepting a connection from the proxy, or run the **`rsgdb replay`**
//! CLI which listens for the proxy to connect.

mod error;
mod jsonl;
mod mock;

pub use error::ReplayError;
pub use jsonl::{load_session, LoadedSession};
pub use mock::{event_to_item, run_mock_backend};
