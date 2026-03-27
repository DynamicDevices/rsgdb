//! rsgdb - Enhanced GDB server/proxy
//!
//! Main entry point for the rsgdb application.

use anyhow::Context;
use clap::{Parser, Subcommand};
use rsgdb::config::Config;
use rsgdb::flash;
use rsgdb::proxy::ProxyServer;
use rsgdb::svd::SvdIndex;
use rsgdb::{init_from_logging_config, LoggingInitGuard};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// Top-level CLI: default command is the GDB proxy; `flash` runs an external programmer from config.
#[derive(Parser, Debug)]
#[command(name = "rsgdb", author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    proxy: ProxyArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Replay a **rsgdb-record v1** JSONL file as a mock TCP backend (issue #10)
    Replay {
        /// Session file (`.jsonl`)
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Mock backend listen address (point `rsgdb` `--target-host` / `--target-port` here)
        #[arg(long, default_value = "127.0.0.1:3334", value_name = "ADDR")]
        listen: String,
    },
    /// Flash firmware using `[flash].program` in config (orchestrates OpenOCD, probe-rs, etc.)
    Flash {
        /// Firmware image (binary, ELF, or whatever your tool expects)
        #[arg(value_name = "IMAGE")]
        image: PathBuf,
        /// Configuration file (`[flash]` section)
        #[arg(short, long)]
        config: Option<PathBuf>,
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long)]
        debug: bool,
    },
}

/// Proxy mode — used when no subcommand is given.
#[derive(Parser, Debug)]
struct ProxyArgs {
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

    /// CMSIS-SVD file path (peripheral/register labels for memory RSP in logs)
    #[arg(long, value_name = "FILE")]
    svd: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Replay { file, listen }) => run_replay(&file, &listen).await,
        Some(Commands::Flash {
            image,
            config,
            verbose,
            debug,
        }) => run_flash_main(&image, config.as_deref(), verbose, debug),
        None => run_proxy(cli.proxy).await,
    }
}

async fn run_replay(file: &Path, listen: &str) -> anyhow::Result<()> {
    use rsgdb::replay::{load_session, run_mock_backend};
    use tokio::net::TcpListener;

    let mut config = Config::default();
    config.merge_env();
    let _log_guard: LoggingInitGuard = init_from_logging_config(&config.logging, false, false)
        .map_err(|e| anyhow::anyhow!("logging init: {}", e))?;

    let session = load_session(file).with_context(|| format!("load {}", file.display()))?;
    let addr: std::net::SocketAddr = listen
        .parse()
        .with_context(|| format!("parse listen address {:?}", listen))?;
    let listener = TcpListener::bind(addr).await?;
    info!(
        path = %file.display(),
        addr = %listener.local_addr()?,
        events = session.events.len(),
        "Replay mock backend — connect rsgdb with e.g. --target-host 127.0.0.1 --target-port {}",
        listener.local_addr()?.port()
    );

    loop {
        let (sock, peer) = listener.accept().await?;
        info!("Replay: backend connection from {}", peer);
        let events = session.events.clone();
        tokio::spawn(async move {
            if let Err(e) = run_mock_backend(sock, events).await {
                error!("replay session ended: {}", e);
            }
        });
    }
}

fn run_flash_main(
    image: &Path,
    config_path: Option<&Path>,
    verbose: bool,
    debug: bool,
) -> anyhow::Result<()> {
    let mut config = if let Some(path) = config_path {
        Config::from_file(path)?
    } else {
        Config::default()
    };

    config.merge_env();

    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let _log_guard: LoggingInitGuard = init_from_logging_config(&config.logging, verbose, debug)
        .map_err(|e| anyhow::anyhow!("logging init: {}", e))?;

    info!(
        image = %image.display(),
        "Flash orchestration"
    );

    flash::run_flash(&config.flash, image).context("flash command failed")?;

    info!("Flash finished successfully");
    Ok(())
}

async fn run_proxy(args: ProxyArgs) -> anyhow::Result<()> {
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
    if let Some(svd) = &args.svd {
        config.svd.path = Some(svd.to_string_lossy().into_owned());
    }

    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let svd_index: Option<Arc<SvdIndex>> = match config
        .svd
        .path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(path) => {
            let idx = SvdIndex::load_from_path(Path::new(path))
                .with_context(|| format!("loading SVD {}", path))?;
            info!(
                path = %path,
                registers = idx.register_count(),
                "Loaded CMSIS-SVD"
            );
            Some(Arc::new(idx))
        }
        None => None,
    };

    let _log_guard: LoggingInitGuard =
        init_from_logging_config(&config.logging, args.verbose, args.debug)
            .map_err(|e| anyhow::anyhow!("logging init: {}", e))?;

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

    let mut server =
        ProxyServer::new(config.proxy.clone(), config.recording.clone(), svd_index).await?;

    info!(
        listen = %server.local_addr()?,
        "Proxy server listening for GDB connections"
    );

    server.run().await.map_err(|e| {
        error!("Server error: {}", e);
        anyhow::anyhow!("{}", e)
    })?;
    Ok(())
}
