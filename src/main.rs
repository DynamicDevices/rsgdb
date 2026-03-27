//! rsgdb - Enhanced GDB Server/Proxy CLI

use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "rsgdb")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set the logging level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Configuration file path
    #[arg(short, long, default_value = "rsgdb.toml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the GDB proxy server
    Serve {
        /// Port to listen on for GDB connections
        #[arg(short, long, default_value = "3333")]
        port: u16,

        /// Backend type (openocd, probe-rs, pyocd)
        #[arg(short, long, default_value = "openocd")]
        backend: String,

        /// Target host
        #[arg(long, default_value = "localhost")]
        target_host: String,

        /// Target port
        #[arg(long, default_value = "3334")]
        target_port: u16,
    },

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = match cli.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("rsgdb v{} starting", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Some(Commands::Serve {
            port,
            backend,
            target_host,
            target_port,
        }) => {
            info!("Starting GDB proxy server");
            info!("  Listen port: {}", port);
            info!("  Backend: {}", backend);
            info!("  Target: {}:{}", target_host, target_port);

            // TODO: Implement proxy server
            println!("Proxy server functionality not yet implemented");
            println!("This will start the GDB proxy on port {}", port);
        }
        Some(Commands::Version) => {
            println!("rsgdb version {}", env!("CARGO_PKG_VERSION"));
            println!("Rust version: {}", env!("CARGO_PKG_RUST_VERSION"));
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

// Made with Bob
