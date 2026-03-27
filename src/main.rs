//! rsgdb - Enhanced GDB server/proxy
//!
//! Main entry point for the rsgdb application.

use clap::Parser;
use rsgdb::config::Config;
use rsgdb::proxy::ProxyServer;
use rsgdb::{init_from_logging_config, LoggingInitGuard};
use std::path::PathBuf;
use tracing::{error, info};

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Listen port for GDB connections
    #[arg(short, long)]
    port: Option<u16>,

    /// Target host to connect to
    #[arg(short, long)]
    target_host: Option<String>,

    /// Target port to connect to
    #[arg(short = 'P', long)]
    target_port: Option<u16>,

    /// Debug backend type (openocd, probe-rs, pyocd); stored in config for future use
    #[arg(long)]
    backend: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Record RSP traffic to JSONL (rsgdb-record v1) under `recording.output_dir`
    #[arg(long)]
    record: bool,

    /// Override recording output directory (implies recording if set)
    #[arg(long, value_name = "DIR")]
    record_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let mut config = if let Some(config_path) = &args.config {
        Config::from_file(config_path)?
    } else {
        Config::default()
    };

    config.merge_env();

    if let Some(port) = args.port {
        config.proxy.listen_port = port;
    }
    if let Some(target_host) = args.target_host {
        config.proxy.target_host = target_host;
    }
    if let Some(target_port) = args.target_port {
        config.proxy.target_port = target_port;
    }
    if let Some(backend) = args.backend {
        config.backend.backend_type = backend;
    }

    if args.record {
        config.recording.enabled = true;
    }
    if let Some(dir) = args.record_dir {
        config.recording.output_dir = dir.to_string_lossy().into_owned();
        config.recording.enabled = true;
    }

    config.validate()?;

    let _log_guard: LoggingInitGuard =
        init_from_logging_config(&config.logging, args.verbose, args.debug)?;

    info!("Starting rsgdb v{}", env!("CARGO_PKG_VERSION"));
    if let Some(config_path) = &args.config {
        info!("Loaded configuration from {:?}", config_path);
    } else {
        info!("Using default configuration");
    }

    info!(
        backend_type = %config.backend.backend_type,
        "Configured debug backend (for future integration)"
    );

    info!("Configuration: {:?}", config);

    let mut server = ProxyServer::new(config.proxy.clone(), config.recording.clone()).await?;

    info!(
        listen = %server.local_addr()?,
        "Proxy server listening for GDB connections"
    );

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}
