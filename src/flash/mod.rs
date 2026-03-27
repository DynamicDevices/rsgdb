//! Flash programming orchestration — run an external tool (OpenOCD, probe-rs, etc.) with `{image}` substitution.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::FlashConfig;

/// Expand `{image}` in each configured argv element using the canonical firmware path.
pub fn build_argv(template: &[String], image: &Path) -> Result<Vec<String>> {
    if !image.exists() {
        anyhow::bail!("firmware image not found: {}", image.display());
    }
    let image_str = image
        .canonicalize()
        .with_context(|| format!("resolving image path {}", image.display()))?
        .to_string_lossy()
        .into_owned();

    if !template.iter().any(|s| s.contains("{image}")) {
        anyhow::bail!(
            "[flash].program must include {{image}} somewhere so the firmware path is passed to your tool"
        );
    }

    Ok(template
        .iter()
        .map(|s| s.replace("{image}", &image_str))
        .collect())
}

/// Run the configured flash program and wait for it. Stdin is `/dev/null`; stdout/stderr inherit.
pub fn run_flash(config: &FlashConfig, image: &Path) -> Result<()> {
    if config.program.is_empty() {
        anyhow::bail!(
            "[flash].program is empty; set an argv list in config, e.g. \
             [\"openocd\", \"-f\", \"board.cfg\", \"-c\", \"program {{image}} verify reset exit\"]"
        );
    }

    let argv = build_argv(&config.program, image)?;
    tracing::info!(
        program = %argv[0],
        argc = argv.len(),
        "Running flash orchestration command"
    );

    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.stdin(Stdio::null());

    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn {}", argv[0]))?;

    if !status.success() {
        anyhow::bail!("flash program exited with {}", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn build_argv_substitutes_image() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("fw.bin");
        fs::write(&img, [1, 2, 3]).unwrap();
        let argv = build_argv(
            &[
                "tool".to_string(),
                "-c".to_string(),
                "flash {image} verify".to_string(),
            ],
            &img,
        )
        .unwrap();
        assert_eq!(argv[0], "tool");
        assert!(argv[2].contains("flash "));
        assert!(argv[2].contains("verify"));
        assert!(!argv[2].contains("{image}"));
        assert!(argv[2].contains("fw.bin"));
    }

    #[test]
    fn build_argv_requires_image_placeholder() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("fw.bin");
        fs::write(&img, []).unwrap();
        let err = build_argv(&["openocd".to_string()], &img).unwrap_err();
        assert!(err.to_string().contains("{image}"));
    }
}
