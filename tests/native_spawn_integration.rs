//! End-to-end test: managed native spawn connects to a real TCP listener (Python stdlib).
//! Requires `python3` or `python` on PATH (GitHub Actions runners provide this).

use rsgdb::backends::connect_backend;
use rsgdb::config::{BackendConfig, BackendSpawnConfig, BackendTransport, ProxyConfig};

fn python_exe() -> Option<&'static str> {
    for cmd in ["python3", "python"] {
        let ok = std::process::Command::new(cmd)
            .args(["-c", "import sys; sys.exit(0)"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Some(cmd);
        }
    }
    None
}

#[tokio::test]
async fn native_managed_connects_python_tcp_listener() {
    let py = python_exe().expect("python3 or python required on PATH for this test");
    // Listen on 127.0.0.1:port; block until select timeout (connect from rsgdb succeeds first).
    let script = concat!(
        "import socket,sys;p=int(sys.argv[1]);",
        "s=socket.socket();s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1);",
        "s.bind(('127.0.0.1',p));s.listen(8);",
        "import select;select.select([s],[],[],300)",
    );
    let backend = BackendConfig {
        transport: BackendTransport::Native,
        spawn: BackendSpawnConfig {
            program: vec![py.to_string(), "-c".into(), script.into(), "{port}".into()],
            bind_host: "127.0.0.1".into(),
            ready_timeout_secs: 20,
            poll_interval_ms: 15,
        },
        ..Default::default()
    };

    let proxy = ProxyConfig::default();
    let conn = connect_backend(&proxy, &backend)
        .await
        .expect("connect_backend (native) should reach python listener");

    assert!(conn.spawned_child.is_some());
    drop(conn.framed);
    let mut child = conn.spawned_child.expect("spawned_child");
    let _ = child.kill().await;
    let _ = child.wait().await;
}
