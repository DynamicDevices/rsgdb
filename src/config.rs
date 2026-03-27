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

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend type (openocd, probe-rs, pyocd)
    #[serde(default = "default_backend_type")]
    pub backend_type: String,

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

        // Validate backend config
        let valid_backends = ["openocd", "probe-rs", "pyocd"];
        if !valid_backends.contains(&self.backend.backend_type.as_str()) {
            return Err(ConfigError::InvalidValue {
                field: "backend.backend_type".to_string(),
                reason: format!("Must be one of: {}", valid_backends.join(", ")),
            });
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
}
