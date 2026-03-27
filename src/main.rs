//! rsgdb - Enhanced GDB server/proxy
//!
//! Main entry point for the rsgdb application.

use clap::Parser;
use rsgdb::config::Config;
use rsgdb::proxy::ProxyServer;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    init_tracing(args.verbose, args.debug);

    info!("Starting rsgdb v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let mut config = if let Some(config_path) = args.config {
        info!("Loading configuration from {:?}", config_path);
        Config::from_file(&config_path)?
    } else {
        info!("Using default configuration");
        Config::default()
    };

    // Environment overrides file/defaults; CLI overrides env (see also merge_env docs on Config)
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

    config.validate()?;

    info!("Configuration: {:?}", config);

    // Create and run the proxy server
    let mut server = ProxyServer::new(config.proxy).await?;

    info!("Proxy server started successfully");
    info!("Waiting for GDB connections...");

    // Run the server
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}

/// Initialize tracing/logging
fn init_tracing(verbose: bool, debug: bool) {
    let filter = if debug {
        "rsgdb=debug"
    } else if verbose {
        "rsgdb=info"
    } else {
        "rsgdb=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// Made with Bob
