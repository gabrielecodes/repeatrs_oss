//! Cli interface for repeat.rs

use repeatrs_cli::Config;
use repeatrs_cli::Result;
use repeatrs_cli::{Cli, parse_command};

use clap::Parser;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder().parse_lossy("h2=info");

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .with_env_filter(filter)
        .init();

    let cli = Cli::parse();

    let config = Config::new()?
        .with_request_timeout_ms(cli.request_timeout_ms)
        .with_scheduler_grpc_url(cli.scheduler_grpc_url.clone());

    if let Err(e) = parse_command(cli, config).await {
        eprintln!("Error: {}", e);
    }

    Ok(())
}
