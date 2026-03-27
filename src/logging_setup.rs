//! Install `tracing` from [`crate::config::LoggingConfig`] and CLI verbosity flags.
//!
//! Precedence for effective level: `-d` (debug) > `-v` (info) > config `logging.level` > default.
//!
//! When `logging.output` is set, a [`LoggingInitGuard`] is returned; keep it alive for the
//! process lifetime so the non-blocking writer can flush.

use crate::config::LoggingConfig;
use tracing_subscriber::EnvFilter;

/// Keep this value in `main` while logging to a file (holds the non-blocking worker guard).
pub struct LoggingInitGuard {
    pub(crate) _file_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

/// Effective `rsgdb` log level string for the default env filter.
fn effective_level_str(logging: &LoggingConfig, verbose: bool, debug: bool) -> &str {
    if debug {
        "debug"
    } else if verbose {
        "info"
    } else {
        logging.level.as_str()
    }
}

/// Build default [`EnvFilter`]: honors `RUST_LOG` when set; otherwise derives from config and CLI.
fn build_filter(logging: &LoggingConfig, verbose: bool, debug: bool) -> EnvFilter {
    let level = effective_level_str(logging, verbose, debug);
    let base = if logging.log_protocol {
        format!("rsgdb={level},rsgdb::protocol=trace")
    } else {
        format!("rsgdb={level}")
    };

    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(base))
}

fn install_fmt_with_writer<W>(
    logging: &LoggingConfig,
    filter: EnvFilter,
    writer: W,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    W: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    match (logging.format.as_str(), logging.include_timestamps) {
        ("json", true) => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_thread_ids(logging.include_thread_ids)
                .json()
                .try_init()?;
        }
        ("json", false) => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_thread_ids(logging.include_thread_ids)
                .without_time()
                .json()
                .try_init()?;
        }
        (_, true) => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_thread_ids(logging.include_thread_ids)
                .try_init()?;
        }
        (_, false) => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(writer)
                .with_thread_ids(logging.include_thread_ids)
                .without_time()
                .try_init()?;
        }
    }
    Ok(())
}

/// Initialize global tracing. Call once after configuration is loaded and validated.
///
/// Returns a guard when logging to a file; store it in `main` (e.g. `let _log = ...?`).
pub fn init_from_logging_config(
    logging: &LoggingConfig,
    verbose: bool,
    debug: bool,
) -> Result<LoggingInitGuard, Box<dyn std::error::Error + Send + Sync>> {
    let filter = build_filter(logging, verbose, debug);

    let file_guard = if let Some(path) = &logging.output {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        install_fmt_with_writer(logging, filter, non_blocking)?;
        Some(guard)
    } else {
        install_fmt_with_writer(logging, filter, std::io::stdout)?;
        None
    };

    Ok(LoggingInitGuard {
        _file_guard: file_guard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;

    #[test]
    fn effective_level_cli_debug_wins() {
        let logging = LoggingConfig {
            level: "warn".to_string(),
            ..Default::default()
        };
        assert_eq!(effective_level_str(&logging, false, true), "debug");
    }

    #[test]
    fn effective_level_verbose_over_config() {
        let logging = LoggingConfig {
            level: "warn".to_string(),
            ..Default::default()
        };
        assert_eq!(effective_level_str(&logging, true, false), "info");
    }
}
