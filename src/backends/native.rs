//! Spawn a GDB stub with `{port}` substitution and connect via TCP to `bind_host:port`.

use crate::backends::stream::BackendStream;
use crate::config::BackendSpawnConfig;
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, ChildStderr, Command};
use tokio::sync::Mutex;
use tokio_util::codec::Framed;
use tracing::debug;

/// Max bytes retained from stub stderr (rolling tail) for error messages and logs.
const STDERR_CAPTURE_MAX_BYTES: usize = 12_288;
/// Max bytes appended to a single error message (tail of capture).
const STDERR_ERR_TAIL_BYTES: usize = 2_048;

/// Replace `{port}` in each argv element (same pattern as `[flash].program` / `{image}`).
pub fn build_spawn_argv(program: &[String], port: u16) -> Result<Vec<String>, RsgdbError> {
    if !program.iter().any(|s| s.contains("{port}")) {
        return Err(RsgdbError::Backend(
            "[backend.spawn] program must include the placeholder {port}".into(),
        ));
    }
    Ok(program
        .iter()
        .map(|s| s.replace("{port}", &port.to_string()))
        .collect())
}

/// Reserve an ephemeral TCP port on `spawn.bind_host` (listener is dropped before the stub binds).
pub fn pick_ephemeral_port(spawn: &BackendSpawnConfig) -> Result<u16, RsgdbError> {
    let listener =
        std::net::TcpListener::bind((spawn.bind_host.as_str(), 0u16)).map_err(RsgdbError::Io)?;
    let port = listener.local_addr().map_err(RsgdbError::Io)?.port();
    drop(listener);
    Ok(port)
}

fn utf8_tail(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut start = s.len() - max_bytes;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    &s[start..]
}

pub(crate) fn enrich_err_with_stderr(err: RsgdbError, stderr: &str) -> RsgdbError {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        return err;
    }
    let tail = utf8_tail(stderr, STDERR_ERR_TAIL_BYTES);
    match err {
        RsgdbError::Timeout(msg) => {
            RsgdbError::Timeout(format!("{msg}\n--- stub stderr (recent) ---\n{tail}"))
        }
        RsgdbError::Backend(msg) => {
            RsgdbError::Backend(format!("{msg}\n--- stub stderr ---\n{tail}"))
        }
        other => other,
    }
}

pub(crate) async fn snapshot_stderr_capture(capture: &Arc<Mutex<String>>) -> String {
    tokio::time::sleep(Duration::from_millis(100)).await;
    capture.lock().await.clone()
}

pub(crate) async fn drain_stub_stderr(stderr: ChildStderr, capture: Arc<Mutex<String>>) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end();
                tracing::debug!(
                    target: "rsgdb::stub_stderr",
                    line = %trimmed,
                    "stub stderr"
                );
                let mut cap = capture.lock().await;
                if !cap.is_empty() {
                    cap.push('\n');
                }
                cap.push_str(trimmed);
                if cap.len() > STDERR_CAPTURE_MAX_BYTES {
                    let excess = cap.len() - STDERR_CAPTURE_MAX_BYTES;
                    cap.drain(..excess);
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "stub stderr read ended");
                break;
            }
        }
    }
}

/// Wait until `TcpStream::connect` succeeds, the child exits, or `timeout` elapses.
pub async fn wait_for_tcp_connect(
    host: &str,
    port: u16,
    timeout: Duration,
    poll: Duration,
    child: &mut Child,
) -> Result<TcpStream, RsgdbError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(RsgdbError::Backend(format!(
                "stub process exited before TCP was ready on {host}:{port} (status={status}); \
                 check [backend.spawn] argv, PATH, and that the program listens on {host} at the substituted port"
            )));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(RsgdbError::Timeout(format!(
                "stub did not accept TCP on {host}:{port} within {timeout:?}; \
                 confirm the stub binds to {host} and uses the same port as `{port}` in argv; \
                 increase [backend.spawn] ready_timeout_secs if the tool starts slowly"
            )));
        }
        match TcpStream::connect((host, port)).await {
            Ok(s) => return Ok(s),
            Err(_) => tokio::time::sleep(poll).await,
        }
    }
}

async fn spawn_managed_stub(
    spawn: &BackendSpawnConfig,
    port: u16,
) -> Result<(Child, Arc<Mutex<String>>), RsgdbError> {
    let argv = build_spawn_argv(&spawn.program, port)?;
    if argv.is_empty() {
        return Err(RsgdbError::Backend("spawn program is empty".into()));
    }
    debug!(
        bind_host = %spawn.bind_host,
        port,
        argv = ?argv,
        "spawning native GDB stub subprocess"
    );
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::inherit());
    let mut child = cmd.spawn().map_err(|e| {
        RsgdbError::Backend(format!(
            "failed to spawn stub `{}`: {e}; check PATH and [backend.spawn] program",
            argv[0]
        ))
    })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| RsgdbError::Backend("internal error: stub stderr not piped".into()))?;
    let capture = Arc::new(Mutex::new(String::new()));
    let cap = capture.clone();
    tokio::spawn(async move {
        drain_stub_stderr(stderr, cap).await;
    });
    Ok((child, capture))
}

pub async fn connect_native_managed(
    spawn: &BackendSpawnConfig,
) -> Result<(Framed<BackendStream, GdbCodec>, Child), RsgdbError> {
    let port = pick_ephemeral_port(spawn)?;
    let (mut child, stderr_cap) = spawn_managed_stub(spawn, port).await?;
    let timeout = Duration::from_secs(spawn.ready_timeout_secs.max(1));
    let poll = Duration::from_millis(spawn.poll_interval_ms.max(10));
    let stream = match wait_for_tcp_connect(&spawn.bind_host, port, timeout, poll, &mut child).await
    {
        Ok(s) => s,
        Err(e) => {
            let stderr = snapshot_stderr_capture(&stderr_cap).await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(enrich_err_with_stderr(e, &stderr));
        }
    };
    Ok((
        Framed::new(BackendStream::Tcp(stream), GdbCodec::new()),
        child,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_spawn_argv_replaces_port() {
        let v = build_spawn_argv(&["sh".into(), "-c".into(), "echo {port}".into()], 4242).unwrap();
        assert_eq!(v, vec!["sh", "-c", "echo 4242"]);
    }

    #[test]
    fn build_spawn_argv_requires_placeholder() {
        let err = build_spawn_argv(&["true".into()], 1).unwrap_err();
        match err {
            RsgdbError::Backend(s) => assert!(s.contains("{port}")),
            e => panic!("unexpected {e:?}"),
        }
    }

    #[test]
    fn pick_ephemeral_port_loopback() {
        let spawn = BackendSpawnConfig {
            bind_host: "127.0.0.1".to_string(),
            ..Default::default()
        };
        let p = pick_ephemeral_port(&spawn).unwrap();
        assert!(p > 0);
    }

    #[test]
    fn enrich_appends_stderr_tail() {
        let e = RsgdbError::Backend("main".into());
        let out = enrich_err_with_stderr(e, "line1\nline2");
        match out {
            RsgdbError::Backend(s) => {
                assert!(s.contains("main"));
                assert!(s.contains("line1"));
                assert!(s.contains("stub stderr"));
            }
            _ => panic!("expected Backend"),
        }
    }

    /// Stub exits immediately — `wait_for_tcp_connect` should report exit, not hang until timeout.
    #[cfg(unix)]
    #[tokio::test]
    async fn connect_native_managed_errors_when_stub_exits_before_listen() {
        let spawn = BackendSpawnConfig {
            program: vec![
                "sh".into(),
                "-c".into(),
                "echo rsgdb_stderr_marker >&2; exit 0".into(),
                "{port}".into(),
            ],
            ready_timeout_secs: 15,
            poll_interval_ms: 5,
            ..Default::default()
        };
        let err = connect_native_managed(&spawn).await.unwrap_err();
        match err {
            RsgdbError::Backend(msg) => {
                assert!(
                    msg.contains("exited"),
                    "expected stub exit error, got: {msg}"
                );
                assert!(
                    msg.contains("rsgdb_stderr_marker"),
                    "expected stderr in error, got: {msg}"
                );
            }
            e => panic!("unexpected {e:?}"),
        }
    }
}
