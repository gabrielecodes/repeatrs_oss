use crate::error::Result;

use chrono::DateTime;
use clap::Args;
use clap::Subcommand;
use repeatrs_proto::repeatrs::{Empty, QueueItem, QueueListResponse};
use repeatrs_proto::repeatrs::{
    QueueIdentifierRequest, grpc_queue_service_client::GrpcQueueServiceClient,
    queue_identifier_request::QueueIdentifier,
};
use tabled::{Table, Tabled, settings::Style};
use tonic::transport::Channel;
use tonic::{Request, Response};
use uuid::Uuid;

#[derive(Debug, Args)]
#[clap(group(
     clap::ArgGroup::new("identifier")
         .required(true).args(&["queue_id", "queue_name"])
 ))]
pub struct QueueIdentifierArgs {
    /// UUID of the queue
    #[arg(long)]
    pub job_id: Option<Uuid>,

    /// Name of the queue
    #[arg(long)]
    pub job_name: Option<String>,
}

impl std::fmt::Display for QueueIdentifierArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.job_id {
            write!(f, "id: {}", id)
        } else if let Some(name) = &self.job_name {
            write!(f, "name: {}", name)
        } else {
            // this case should be prevented by clap's ArgGroup
            write!(f, "unspecified identifier")
        }
    }
}

impl From<QueueIdentifierArgs> for QueueIdentifierRequest {
    fn from(value: QueueIdentifierArgs) -> Self {
        match (value.job_id, value.job_name) {
            (Some(id), Some(_name)) => QueueIdentifierRequest {
                queue_identifier: Some(QueueIdentifier::QueueId(id.to_string())),
            },
            (Some(id), None) => QueueIdentifierRequest {
                queue_identifier: Some(QueueIdentifier::QueueId(id.to_string())),
            },
            (None, Some(name)) => QueueIdentifierRequest {
                queue_identifier: Some(QueueIdentifier::QueueName(name)),
            },
            (None, None) => QueueIdentifierRequest {
                queue_identifier: None,
            },
        }
    }
}

/// Commands for managing queues
#[derive(Debug, Subcommand)]
pub enum QueueSubcommand {
    // List queues
    #[command(alias = "ls", about = "lists all queues")]
    List,
}

pub struct QueueService {
    client: GrpcQueueServiceClient<Channel>,
}

impl QueueService {
    pub fn with_channel(channel: Channel) -> Self {
        let client = GrpcQueueServiceClient::new(channel);
        Self { client }
    }

    pub async fn handle_command(self, command: QueueSubcommand) -> Result<()> {
        match command {
            QueueSubcommand::List => self.list_queues().await?,
        }
        Ok(())
    }

    async fn list_queues(mut self) -> Result<()> {
        let response: Response<QueueListResponse> =
            self.client.list_queues(Request::new(Empty {})).await?;

        let queues = response.get_ref();

        let display_data: Vec<QueueDisplay> =
            queues.queues.iter().map(QueueDisplay::from).collect();

        if !queues.queues.is_empty() {
            let table = Table::new(&display_data).with(Style::blank()).to_string();
            println!("{}", table);
        } else {
            println!("No queues found.");
        }

        Ok(())
    }
}

#[derive(Tabled)]
pub struct QueueDisplay {
    #[tabled(rename = "NAME")]
    pub queue_name: String,

    #[tabled(rename = "STATUS")]
    pub status: String,

    #[tabled(rename = "CAPACITY")]
    pub capacity: i32,

    #[tabled(rename = "JOBS")]
    pub used_capacity: i32,

    #[tabled(rename = "CREATED AT")]
    pub created_at: String,
}

impl From<&QueueItem> for QueueDisplay {
    fn from(item: &QueueItem) -> Self {
        let date_str = item
            .created_at
            .as_ref()
            .map(|ts| {
                DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Invalid Date".to_string())
            })
            .unwrap_or_else(|| "N/A".to_string());

        Self {
            queue_name: item.queue_name.clone(),
            status: item.status.clone(),
            capacity: item.capacity,
            used_capacity: item.used_capacity,
            created_at: date_str,
        }
    }
}
