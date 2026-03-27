//! Configuration management for rsgdb

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{ConfigError, ConfigResult};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Proxy configuration
    #[serde(default)]
    pub proxy: ProxyConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Breakpoint configuration
    #[serde(default)]
    pub breakpoints: BreakpointConfig,

    /// Backend configuration
    #[serde(default)]
    pub backend: BackendConfig,

    /// Session recording configuration
    #[serde(default)]
    pub recording: RecordingConfig,

    /// CMSIS-SVD file for read-only peripheral/register annotation (memory RSP)
    #[serde(default)]
    pub svd: SvdConfig,

    /// External flash programming command (`rsgdb flash`)
    #[serde(default)]
    pub flash: FlashConfig,
}

/// argv template for `rsgdb flash`; each string may contain `{image}` (replaced by the firmware path).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlashConfig {
    #[serde(default)]
    pub program: Vec<String>,
}

/// Optional CMSIS-SVD path. When set, memory `m`/`M` packets are annotated in logs (`target: rsgdb::svd`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvdConfig {
    /// Path to `.svd` XML (device description)
    #[serde(default)]
    pub path: Option<String>,
}

/// Proxy server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port to listen on for GDB connections
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,

    /// Target host to connect to
    #[serde(default = "default_target_host")]
    pub target_host: String,

    /// Target port to connect to
    #[serde(default = "default_target_port")]
    pub target_port: u16,

    /// Reserved for future RSP ack policy; the proxy always forwards `+`/`-` today.
    #[serde(default = "default_true")]
    pub enable_acks: bool,

    /// Max time to establish the TCP connection to the backend (`0` = no limit). Does not apply
    /// to idle GDB sessions (no read timeout).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (text, json)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Output file path (None for stdout)
    pub output: Option<String>,

    /// Log all protocol traffic
    #[serde(default = "default_true")]
    pub log_protocol: bool,

    /// Include timestamps
    #[serde(default = "default_true")]
    pub include_timestamps: bool,

    /// Include thread IDs
    #[serde(default = "default_false")]
    pub include_thread_ids: bool,
}

/// Breakpoint management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointConfig {
    /// Automatically optimize hardware/software breakpoint usage
    #[serde(default = "default_true")]
    pub auto_optimize: bool,

    /// Maximum number of hardware breakpoints
    #[serde(default = "default_max_hardware_breakpoints")]
    pub max_hardware: u32,

    /// Enable named breakpoints
    #[serde(default = "default_true")]
    pub enable_named: bool,

    /// Enable conditional breakpoints
    #[serde(default = "default_true")]
    pub enable_conditional: bool,
}

/// How the proxy reaches the debug target (stub TCP vs future native probe).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendTransport {
    /// GDB remote stub on `proxy.target_host`:`proxy.target_port` (OpenOCD, probe-rs GDB port, …).
    #[default]
    #[serde(alias = "stub")]
    Tcp,
    /// rsgdb **spawns** a GDB stub (e.g. probe-rs, OpenOCD) with `{port}` in argv, then connects via TCP.
    Native,
    /// SSH to `target_host` (or `[backend.remote_ssh] host`), run remote argv (e.g. `gdbserver`), then TCP to `proxy.target_host`:`proxy.target_port`.
    #[serde(rename = "remote_ssh", alias = "ssh")]
    RemoteSsh,
}

impl BackendTransport {
    /// Parse from config strings (`tcp`, `stub`, `native`, `remote_ssh`, `ssh`).
    pub fn parse(s: &str) -> ConfigResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tcp" | "stub" => Ok(Self::Tcp),
            "native" => Ok(Self::Native),
            "remote_ssh" | "ssh" => Ok(Self::RemoteSsh),
            "" => Err(ConfigError::InvalidValue {
                field: "backend.transport".to_string(),
                reason: "Cannot be empty".to_string(),
            }),
            other => Err(ConfigError::InvalidValue {
                field: "backend.transport".to_string(),
                reason: format!("Must be tcp, native, or remote_ssh, got: {other}"),
            }),
        }
    }
}

impl std::str::FromStr for BackendTransport {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).map_err(|e| e.to_string())
    }
}

/// argv template for `transport = native`: must include `{port}` (ephemeral port rsgdb allocates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSpawnConfig {
    /// Stub command line; `{port}` is replaced once per GDB session.
    #[serde(default)]
    pub program: Vec<String>,
    /// Address the stub must listen on (must match argv; default loopback).
    #[serde(default = "default_spawn_bind_host")]
    pub bind_host: String,
    /// Max time to wait for the stub to accept TCP after spawn.
    #[serde(default = "default_spawn_ready_timeout_secs")]
    pub ready_timeout_secs: u64,
    /// Delay between TCP connect attempts while waiting.
    #[serde(default = "default_spawn_poll_ms")]
    pub poll_interval_ms: u64,
}

fn default_spawn_bind_host() -> String {
    "127.0.0.1".to_string()
}

fn default_spawn_ready_timeout_secs() -> u64 {
    30
}

fn default_spawn_poll_ms() -> u64 {
    50
}

impl Default for BackendSpawnConfig {
    fn default() -> Self {
        Self {
            program: Vec::new(),
            bind_host: default_spawn_bind_host(),
            ready_timeout_secs: default_spawn_ready_timeout_secs(),
            poll_interval_ms: default_spawn_poll_ms(),
        }
    }
}

/// Remote argv for `transport = remote_ssh`: must include `{port}` (same as `proxy.target_port`).
/// rsgdb runs `ssh user@host …` locally; typically `gdbserver 0.0.0.0:{port} /path/to/binary`.
///
/// **Upload:** if both [`Self::upload_local`] and [`Self::upload_remote`] are set, rsgdb runs
/// `scp` before SSH (same credentials as SSH: keys or `RSGDB_SSH_PASSWORD` + `sshpass`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendRemoteSshConfig {
    /// SSH target host; if empty, use `proxy.target_host`.
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub user: String,
    #[serde(default = "default_remote_ssh_port")]
    pub ssh_port: u16,
    #[serde(default)]
    pub identity_file: Option<String>,
    /// Local file to copy to the target before starting gdbserver (e.g. unstripped ELF).
    #[serde(default)]
    pub upload_local: Option<String>,
    /// Destination path on the target (`scp` target); must match the binary path in `program`.
    #[serde(default)]
    pub upload_remote: Option<String>,
    /// Remote command line; `{port}` → `proxy.target_port`.
    #[serde(default)]
    pub program: Vec<String>,
    #[serde(default = "default_spawn_ready_timeout_secs")]
    pub ready_timeout_secs: u64,
    #[serde(default = "default_spawn_poll_ms")]
    pub poll_interval_ms: u64,
}

fn default_remote_ssh_port() -> u16 {
    22
}

impl Default for BackendRemoteSshConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            user: String::new(),
            ssh_port: default_remote_ssh_port(),
            identity_file: None,
            upload_local: None,
            upload_remote: None,
            program: Vec::new(),
            ready_timeout_secs: default_spawn_ready_timeout_secs(),
            poll_interval_ms: default_spawn_poll_ms(),
        }
    }
}

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Label for the stub or tool (openocd, probe-rs, pyocd, …) — logging / future use only.
    #[serde(default = "default_backend_type")]
    pub backend_type: String,

    /// Transport to the target (`tcp` = connect to existing stub; `native` = spawn stub then TCP).
    #[serde(default)]
    pub transport: BackendTransport,

    /// Managed stub argv when `transport = native`.
    #[serde(default)]
    pub spawn: BackendSpawnConfig,

    /// SSH + remote argv when `transport = remote_ssh`.
    #[serde(default)]
    pub remote_ssh: BackendRemoteSshConfig,

    /// Backend-specific options
    #[serde(default)]
    pub options: std::collections::HashMap<String, String>,
}

/// Session recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Enable session recording
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Output directory for recordings
    #[serde(default = "default_recording_dir")]
    pub output_dir: String,

    /// Maximum recording size in MB
    #[serde(default = "default_max_recording_size")]
    pub max_size_mb: u64,

    /// Compress recordings
    #[serde(default = "default_true")]
    pub compress: bool,
}

// Default value functions
fn default_listen_port() -> u16 {
    3333
}
fn default_target_host() -> String {
    "localhost".to_string()
}
fn default_target_port() -> u16 {
    3334
}
fn default_timeout() -> u64 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "text".to_string()
}
fn default_max_hardware_breakpoints() -> u32 {
    6
}
fn default_backend_type() -> String {
    "openocd".to_string()
}
fn default_recording_dir() -> String {
    "./recordings".to_string()
}
fn default_max_recording_size() -> u64 {
    100
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            listen_port: default_listen_port(),
            target_host: default_target_host(),
            target_port: default_target_port(),
            enable_acks: default_true(),
            timeout_secs: default_timeout(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: None,
            log_protocol: default_true(),
            include_timestamps: default_true(),
            include_thread_ids: default_false(),
        }
    }
}

impl Default for BreakpointConfig {
    fn default() -> Self {
        Self {
            auto_optimize: default_true(),
            max_hardware: default_max_hardware_breakpoints(),
            enable_named: default_true(),
            enable_conditional: default_true(),
        }
    }
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: default_backend_type(),
            transport: BackendTransport::default(),
            spawn: BackendSpawnConfig::default(),
            remote_ssh: BackendRemoteSshConfig::default(),
            options: std::collections::HashMap::new(),
        }
    }
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            output_dir: default_recording_dir(),
            max_size_mb: default_max_recording_size(),
            compress: default_true(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let contents =
            fs::read_to_string(path).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        let config: Config =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a TOML string
    pub fn from_toml_str(s: &str) -> ConfigResult<Self> {
        let config: Config =
            toml::from_str(s).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;

        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        fs::write(path, contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> ConfigResult<()> {
        // listen_port may be 0 for ephemeral bind (tests, dynamic port)

        if self.proxy.target_port == 0 {
            return Err(ConfigError::InvalidValue {
                field: "proxy.target_port".to_string(),
                reason: "Port cannot be 0".to_string(),
            });
        }

        if self.proxy.target_host.is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "proxy.target_host".to_string(),
                reason: "Host cannot be empty".to_string(),
            });
        }

        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "logging.level".to_string(),
                reason: format!("Must be one of: {}", valid_levels.join(", ")),
            });
        }

        let valid_formats = ["text", "json"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "logging.format".to_string(),
                reason: format!("Must be one of: {}", valid_formats.join(", ")),
            });
        }

        // Backend label is free-form (openocd, probe-rs, custom); must not be empty.
        if self.backend.backend_type.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "backend.backend_type".to_string(),
                reason: "Cannot be empty".to_string(),
            });
        }

        if self.backend.transport == BackendTransport::Native {
            if self.backend.spawn.program.is_empty() {
                return Err(ConfigError::InvalidValue {
                    field: "backend.spawn.program".to_string(),
                    reason: "Required when transport = native; must include {port} placeholder"
                        .to_string(),
                });
            }
            let joined = self
                .backend
                .spawn
                .program
                .iter()
                .fold(String::new(), |a, s| a + s);
            if !joined.contains("{port}") {
                return Err(ConfigError::InvalidValue {
                    field: "backend.spawn.program".to_string(),
                    reason: "Must contain the substring {port} for the ephemeral GDB stub port"
                        .to_string(),
                });
            }
        }

        if self.backend.transport == BackendTransport::RemoteSsh {
            if self.backend.remote_ssh.user.trim().is_empty() {
                return Err(ConfigError::InvalidValue {
                    field: "backend.remote_ssh.user".to_string(),
                    reason: "Required when transport = remote_ssh".to_string(),
                });
            }
            if self.backend.remote_ssh.program.is_empty() {
                return Err(ConfigError::InvalidValue {
                    field: "backend.remote_ssh.program".to_string(),
                    reason: "Required when transport = remote_ssh; must include {port} placeholder"
                        .to_string(),
                });
            }
            let joined = self
                .backend
                .remote_ssh
                .program
                .iter()
                .fold(String::new(), |a, s| a + s);
            if !joined.contains("{port}") {
                return Err(ConfigError::InvalidValue {
                    field: "backend.remote_ssh.program".to_string(),
                    reason: "Must contain {port} (same as proxy.target_port) for gdbserver bind"
                        .to_string(),
                });
            }

            let ul = self
                .backend
                .remote_ssh
                .upload_local
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let ur = self
                .backend
                .remote_ssh
                .upload_remote
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            match (ul, ur) {
                (Some(_), None) | (None, Some(_)) => {
                    return Err(ConfigError::InvalidValue {
                        field: "backend.remote_ssh.upload".to_string(),
                        reason: "Set both upload_local and upload_remote, or omit both".to_string(),
                    });
                }
                _ => {}
            }
        }

        if self.recording.enabled && self.recording.output_dir.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                field: "recording.output_dir".to_string(),
                reason: "Cannot be empty when recording is enabled".to_string(),
            });
        }

        if let Some(ref p) = self.svd.path {
            let p = p.trim();
            if !p.is_empty() && !Path::new(p).exists() {
                return Err(ConfigError::InvalidValue {
                    field: "svd.path".to_string(),
                    reason: "SVD file not found".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Apply environment variables (`RSGDB_*`). Intended after loading a file; the binary applies
    /// CLI flags after this so **CLI overrides environment**.
    pub fn merge_env(&mut self) {
        if let Ok(port) = std::env::var("RSGDB_PORT") {
            if let Ok(port) = port.parse() {
                self.proxy.listen_port = port;
            }
        }

        if let Ok(host) = std::env::var("RSGDB_TARGET_HOST") {
            self.proxy.target_host = host;
        }

        if let Ok(port) = std::env::var("RSGDB_TARGET_PORT") {
            if let Ok(port) = port.parse() {
                self.proxy.target_port = port;
            }
        }

        if let Ok(level) = std::env::var("RSGDB_LOG_LEVEL") {
            self.logging.level = level;
        }

        if let Ok(backend) = std::env::var("RSGDB_BACKEND") {
            self.backend.backend_type = backend;
        }

        if let Ok(t) = std::env::var("RSGDB_TRANSPORT") {
            if let Ok(tr) = BackendTransport::parse(&t) {
                self.backend.transport = tr;
            }
        }

        if let Ok(v) = std::env::var("RSGDB_RECORD") {
            let v = v.trim();
            if v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes") {
                self.recording.enabled = true;
            }
        }

        if let Ok(dir) = std::env::var("RSGDB_RECORD_DIR") {
            if !dir.is_empty() {
                self.recording.output_dir = dir;
            }
        }

        if let Ok(path) = std::env::var("RSGDB_SVD") {
            if !path.trim().is_empty() {
                self.svd.path = Some(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.proxy.listen_port, 3333);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_target_port() {
        let mut config = Config::default();
        config.proxy.target_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ephemeral_listen_port_valid() {
        let mut config = Config::default();
        config.proxy.listen_port = 0;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_log_level() {
        let mut config = Config::default();
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_recording_requires_output_dir_when_enabled() {
        let mut config = Config::default();
        config.recording.enabled = true;
        config.recording.output_dir = "   ".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_svd_path_must_exist_when_set() {
        let mut config = Config::default();
        config.svd.path = Some("/nonexistent/absolute/path/device.svd".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_from_toml_str() {
        let toml = r#"
            [proxy]
            listen_port = 4444
            
            [logging]
            level = "debug"
        "#;

        let config = Config::from_toml_str(toml).unwrap();
        assert_eq!(config.proxy.listen_port, 4444);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_native_requires_spawn_program() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::Native;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_native_requires_port_placeholder() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::Native;
        config.backend.spawn.program = vec!["true".into()];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_native_spawn_validates_with_port_placeholder() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::Native;
        config.backend.spawn.program =
            vec!["sh".into(), "-c".into(), "exit 0".into(), "{port}".into()];
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_remote_ssh_requires_user() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::RemoteSsh;
        config.backend.remote_ssh.program = vec![
            "gdbserver".into(),
            "0.0.0.0:{port}".into(),
            "/bin/true".into(),
        ];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_remote_ssh_validates_with_user_and_port_placeholder() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::RemoteSsh;
        config.backend.remote_ssh.user = "root".into();
        config.backend.remote_ssh.program = vec![
            "gdbserver".into(),
            "0.0.0.0:{port}".into(),
            "/bin/true".into(),
        ];
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_remote_ssh_upload_requires_both_paths() {
        let mut config = Config::default();
        config.backend.transport = BackendTransport::RemoteSsh;
        config.backend.remote_ssh.user = "u".into();
        config.backend.remote_ssh.program =
            vec!["gdbserver".into(), "0.0.0.0:{port}".into(), "/x".into()];
        config.backend.remote_ssh.upload_local = Some("/a".into());
        assert!(config.validate().is_err());
    }
}
