mod job;
pub mod queue;
pub mod worker;

use crate::Config;
use crate::error::Result;
pub use job::{AddJobArgs, JobIdentifierArgs, JobService, JobSubcommand};
use tokio::time::Duration;
use tonic::transport::{Channel, Endpoint};

#[tracing::instrument(skip_all)]
pub async fn get_grpc_channel(config: &Config) -> Result<Channel> {
    let endpoint = Endpoint::from_shared(config.scheduler_grpc_url())?
        .timeout(Duration::from_millis(config.request_timeout_ms() as u64));

    let channel = endpoint.connect().await?;
    Ok(channel)
}
