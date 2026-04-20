mod commands;
mod config;
mod error;

use crate::commands::get_grpc_channel;
use crate::commands::queue::{QueueService, QueueSubcommand};
pub use crate::commands::{AddJobArgs, JobIdentifierArgs};
pub use crate::commands::{JobService, JobSubcommand};
pub use crate::config::Config;
pub use crate::error::Result;

use clap::{Parser, Subcommand};

/// Cli interface to manage job operations
#[derive(Parser, Debug)]
#[command(
    name = "repeatrs",
    about = "Repeatrs CLI",
    long_about = None,
    version,
    propagate_version = true
)]
pub struct Cli {
    /// URL of the Repeatrs Scheduler
    #[arg(long, env = "REPEATRS_SCHEDULER_GRPC_URL")]
    pub scheduler_grpc_url: Option<String>,

    /// timeout for the gRPC requests
    #[arg(long, env = "REPEATRS_REQUEST_TIMEOUT_MS")]
    pub request_timeout_ms: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
#[non_exhaustive]
pub enum Commands {
    /// Commands to manage jobs
    Job {
        #[command(subcommand)]
        command: Box<JobSubcommand>,
    },

    /// Commands to manage queues
    Queue {
        #[command(subcommand)]
        command: QueueSubcommand,
    }, // Worker management commands
}

#[tracing::instrument(skip_all)]
pub async fn parse_command(cli: Cli, config: Config) -> Result<()> {
    let channel = get_grpc_channel(&config).await?;

    match cli.command {
        Commands::Job { command } => JobService::with_channel(channel).execute(*command).await?,
        Commands::Queue { command } => {
            QueueService::with_channel(channel)
                .handle_command(command)
                .await?
        }
    };

    Ok(())
}
