//! `rsgdb flash` runs `[flash].program` from config with `{image}` substitution.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[cfg(unix)]
#[test]
fn flash_subcommand_runs_configured_stub() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("stub_out.txt");
    let script = dir.path().join("flash_stub.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s' \"$*\" > \"{}\"\nexit 0\n",
            out.display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let img = dir.path().join("fw.bin");
    fs::write(&img, [0x55; 8]).unwrap();

    let cfg = dir.path().join("rsgdb.toml");
    let script_abs = script.canonicalize().unwrap();
    let toml = format!(
        r#"[flash]
program = ["{}", "{{image}}"]
"#,
        toml_escape_path(&script_abs)
    );
    fs::write(&cfg, toml).unwrap();

    let bin = env!("CARGO_BIN_EXE_rsgdb");
    let status = Command::new(bin)
        .args(["flash", "--config"])
        .arg(&cfg)
        .arg(&img)
        .status()
        .expect("spawn rsgdb flash");

    assert!(status.success(), "rsgdb flash should exit 0");

    let recorded = fs::read_to_string(&out).expect("stub output");
    let img_abs = img.canonicalize().unwrap();
    assert!(
        recorded.contains(img_abs.to_str().unwrap()),
        "stub argv should contain image path: {recorded:?}"
    );
}

/// Escape a path for use inside a TOML double-quoted string.
fn toml_escape_path(p: &std::path::Path) -> String {
    p.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
