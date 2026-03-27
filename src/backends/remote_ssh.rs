//! SSH to a remote host, run `gdbserver` (or similar), then connect TCP to `proxy.target_host`:`port`.
//! Optional `scp` upload when `[backend.remote_ssh] upload_local` / `upload_remote` are set.

use crate::backends::native::{
    drain_stub_stderr, enrich_err_with_stderr, snapshot_stderr_capture, wait_for_tcp_connect,
};
use crate::backends::stream::BackendStream;
use crate::config::{BackendConfig, BackendRemoteSshConfig, ProxyConfig};
use crate::error::RsgdbError;
use crate::protocol::codec::GdbCodec;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_util::codec::Framed;
use tracing::debug;

/// Copy `local` to `user@ssh_host:remote` using `scp` (and `sshpass` when `RSGDB_SSH_PASSWORD` is set).
pub async fn scp_upload_to_remote(
    rs: &BackendRemoteSshConfig,
    ssh_host: &str,
    local: &str,
    remote: &str,
) -> Result<(), RsgdbError> {
    let path = Path::new(local);
    if !path.is_file() {
        return Err(RsgdbError::Backend(format!(
            "[backend.remote_ssh] upload_local is not a file: {local}"
        )));
    }

    let user_host = format!("{}@{}", rs.user, ssh_host);
    let dest = format!("{user_host}:{remote}");

    let mut cmd = if let Some(p) = std::env::var("RSGDB_SSH_PASSWORD")
        .ok()
        .filter(|s| !s.is_empty())
    {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(p);
        c.arg("scp");
        c
    } else {
        Command::new("scp")
    };

    cmd.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=15")
        .arg("-P")
        .arg(rs.ssh_port.to_string());

    if let Some(ref key) = rs.identity_file {
        let p = key.trim();
        if !p.is_empty() {
            cmd.arg("-i").arg(p);
        }
    }

    cmd.arg(local);
    cmd.arg(&dest);

    debug!(%local, %dest, "scp upload before remote gdbserver");
    let status = cmd.status().await.map_err(|e| {
        RsgdbError::Backend(format!(
            "failed to run scp (install OpenSSH client scp/sshpass on PATH): {e}"
        ))
    })?;
    if !status.success() {
        return Err(RsgdbError::Backend(format!(
            "scp failed with {status}; check upload_local/upload_remote, SSH auth, and RSGDB_SSH_PASSWORD"
        )));
    }
    Ok(())
}

fn build_remote_ssh_argv(program: &[String], port: u16) -> Result<Vec<String>, RsgdbError> {
    if !program.iter().any(|s| s.contains("{port}")) {
        return Err(RsgdbError::Backend(
            "[backend.remote_ssh] program must include the placeholder {port}".into(),
        ));
    }
    Ok(program
        .iter()
        .map(|s| s.replace("{port}", &port.to_string()))
        .collect())
}

fn build_ssh_command(
    rs: &BackendRemoteSshConfig,
    ssh_host: &str,
    remote_argv: &[String],
) -> Result<Command, RsgdbError> {
    if remote_argv.is_empty() {
        return Err(RsgdbError::Backend(
            "[backend.remote_ssh] program is empty".into(),
        ));
    }

    let user_host = format!("{}@{}", rs.user, ssh_host);

    let mut cmd = if let Some(p) = std::env::var("RSGDB_SSH_PASSWORD")
        .ok()
        .filter(|s| !s.is_empty())
    {
        let mut c = Command::new("sshpass");
        c.arg("-p").arg(p);
        c.arg("ssh");
        c
    } else {
        Command::new("ssh")
    };

    cmd.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=15")
        .arg("-p")
        .arg(rs.ssh_port.to_string());

    if let Some(ref path) = rs.identity_file {
        let p = path.trim();
        if !p.is_empty() {
            cmd.arg("-i").arg(p);
        }
    }

    cmd.arg(&user_host);
    for a in remote_argv {
        cmd.arg(a);
    }

    Ok(cmd)
}

pub async fn connect_remote_ssh(
    proxy: &ProxyConfig,
    backend: &BackendConfig,
) -> Result<(Framed<BackendStream, GdbCodec>, Child), RsgdbError> {
    let rs = &backend.remote_ssh;
    let ssh_host = if rs.host.trim().is_empty() {
        proxy.target_host.as_str()
    } else {
        rs.host.as_str()
    };

    let port = proxy.target_port;
    let remote_argv = build_remote_ssh_argv(&rs.program, port)?;

    if let (Some(ref loc), Some(ref rem)) = (&rs.upload_local, &rs.upload_remote) {
        let loc = loc.trim();
        let rem = rem.trim();
        if !loc.is_empty() && !rem.is_empty() {
            scp_upload_to_remote(rs, ssh_host, loc, rem).await?;
        }
    }

    let mut cmd = build_ssh_command(rs, ssh_host, &remote_argv)?;
    debug!(
        ssh_host = %ssh_host,
        tcp_connect = %format!("{}:{}", proxy.target_host, port),
        argv = ?remote_argv,
        "spawning remote gdbserver via SSH"
    );

    cmd.stdin(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::inherit());

    let mut child = cmd.spawn().map_err(|e| {
        RsgdbError::Backend(format!(
            "failed to spawn ssh to {ssh_host}: {e}; install OpenSSH client, check PATH, and RSGDB_SSH_PASSWORD if using password auth"
        ))
    })?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| RsgdbError::Backend("internal error: ssh stderr not piped".into()))?;
    let capture = Arc::new(Mutex::new(String::new()));
    let cap = capture.clone();
    tokio::spawn(async move {
        drain_stub_stderr(stderr, cap).await;
    });

    let timeout = Duration::from_secs(rs.ready_timeout_secs.max(1));
    let poll = Duration::from_millis(rs.poll_interval_ms.max(10));

    let tcp_host = proxy.target_host.as_str();
    let stream = match wait_for_tcp_connect(tcp_host, port, timeout, poll, &mut child).await {
        Ok(s) => s,
        Err(e) => {
            let stderr = snapshot_stderr_capture(&capture).await;
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
    fn build_remote_ssh_argv_replaces_port() {
        let v = build_remote_ssh_argv(
            &[
                "gdbserver".into(),
                "0.0.0.0:{port}".into(),
                "/bin/true".into(),
            ],
            2345,
        )
        .unwrap();
        assert_eq!(v, vec!["gdbserver", "0.0.0.0:2345", "/bin/true"]);
    }

    #[tokio::test]
    async fn scp_upload_missing_local_errors() {
        let rs = BackendRemoteSshConfig::default();
        let err = scp_upload_to_remote(&rs, "127.0.0.1", "/nonexistent/rsgdb_scp_test", "/tmp/x")
            .await
            .unwrap_err();
        match err {
            RsgdbError::Backend(s) => assert!(s.contains("upload_local")),
            e => panic!("unexpected {e:?}"),
        }
    }
}
