// pub mod add;
// pub mod deactivate;
// pub mod delete;

use crate::commands::queue::QueueIdentifierArgs;
use crate::error::{Error, Result};

use clap::{Args, Subcommand};
use croner::{Cron, errors::CronError};
use repeatrs_proto::repeatrs::QueueIdentifierRequest;
use repeatrs_proto::repeatrs::{
    AddJobRequest, JobIdentifierRequest, JobItem, JobServiceResponse,
    grpc_job_service_client::GrpcJobServiceClient, job_identifier_request::JobIdentifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::Tabled;
use tonic::transport::Channel;
use tonic::{Request, Response};
use uuid::Uuid;

#[derive(Debug, Args, Serialize, Deserialize)]
pub struct AddJobArgs {
    /// Path to a toml job configuration file
    #[arg(short, long, value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Name of the job
    #[arg(short, long = "name", required_unless_present = "file")]
    job_name: Option<String>,

    #[arg(long, help = "the description of the job")]
    description: Option<String>,

    /// Schedule
    #[arg(
        short = 's',
        long,
        // value_parser = parse_cron,
        help = "a 5-7 fields cron schedule between single or double quotes (e.g. '* * * * *')",
        required_unless_present = "file"
        )]
    schedule: Option<String>,

    #[arg(
        short = 'o',
        long,
        help = "space delimited option flags that configure how the container is run"
    )]
    options: Option<String>,

    /// Image repository
    #[arg(
        short,
        long,
        help = "the image name as <optional_registry>/<repository>:<tag>",
        required_unless_present = "file"
    )]
    image_name: Option<String>,

    /// Command to execute
    #[arg(
        short = 'm',
        long,
        help = "name of the executable that runs inside the container"
    )]
    command: Option<String>,

    #[arg(
        short = 'x',
        long,
        help = "space delimited arguments passed to the command",
        value_delimiter = ' '
    )]
    args: Option<Vec<String>>,

    /// Retry on failure
    #[arg(long, help = "retry on failure (true/false)")]
    no_retry: Option<bool>,

    #[arg(short, long, help = "job priority")]
    priority: Option<i32>,

    #[arg(
        long,
        help = "name of the queue this job is assigned to",
        default_value = "default"
    )]
    queue_name: Option<String>,

    /// Maximum number of running jobs concurrentöl
    #[arg(long, help = "maximum number of running jobs concurrently")]
    max_concurrency: Option<i32>,

    #[arg(
        long,
        help = "time duration in seconds before the job is forcefully stopped"
    )]
    timeout_seconds: Option<i32>,

    /// If true, instead of calling the gRPC create_job, you just print the JSON
    /// of the CreateJobRequest to the terminal.
    #[arg(
        long,
        help = "prints the arguments that are used to configure this job"
    )]
    dry_run: bool,
}

// fn is_uuid(s: &str) -> Result<Uuid> {
//     Uuid::from_str(s).map_err(|e| Error::Config(e.to_string()))
// }

fn parse_cron(s: &str) -> std::result::Result<Cron, CronError> {
    Cron::from_str(s)
}

impl Display for AddJobArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO print as table
        let json = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", json)
    }
}

impl TryFrom<AddJobArgs> for AddJobRequest {
    type Error = crate::error::Error;

    fn try_from(value: AddJobArgs) -> std::result::Result<Self, Self::Error> {
        let name = value
            .job_name
            .ok_or_else(|| Error::Cli("Missing job name"))?;

        let schedule_str = value.schedule.map(|s| {
            let _ = parse_cron(&s).expect("Invalid cron expression");
            s.to_string()
        });

        let schedule = schedule_str.ok_or_else(|| Error::Cli("Missing job schedule"))?;

        let image_name = value
            .image_name
            .ok_or_else(|| Error::Cli("Missing image name"))?;

        let queue_name = value
            .queue_name
            .ok_or_else(|| Error::Cli("Missing queue name"))?;

        let req = AddJobRequest {
            job_name: name,
            description: value.description,
            schedule,
            image_name,
            queue_name,
            no_retry: value.no_retry.unwrap_or(false),
            options: value.options,
            command: value.command,
            args: value.args.unwrap_or_default(),
            priority: value.priority.unwrap_or(1),
            max_concurrency: value.max_concurrency.unwrap_or(100),
            timeout_seconds: value.timeout_seconds.unwrap_or(7200),
        };

        Ok(req)
    }
}

impl AddJobArgs {
    pub fn default(&self) -> Self {
        Self {
            file: None,
            job_name: self.job_name.to_owned(),
            description: self.description.to_owned(),
            schedule: self.schedule.to_owned(),
            image_name: self.image_name.to_owned(),
            queue_name: self.queue_name.to_owned(),
            no_retry: Some(false),
            options: self.options.to_owned(),
            command: self.command.to_owned(),
            args: None,
            priority: Some(1),
            max_concurrency: Some(-1),
            timeout_seconds: Some(7200),
            dry_run: false,
        }
    }

    /// Merges [AddJobArgs] provided via cli command with those provided in the job
    /// definition file, giving precedence to the cli arguments.
    #[tracing::instrument(skip_all, fields(file = ?self.file))]
    pub fn resolve(self) -> Result<AddJobRequest> {
        let Some(path) = &self.file else {
            let req: AddJobRequest = self.try_into()?;
            return Ok(req);
        };

        let content = std::fs::read_to_string(path)?;
        let parsed: AddJobArgs = toml::from_str(&content)?;

        let job_name = self
            .job_name
            .or(parsed.job_name)
            .ok_or_else(|| Error::Cli("Missing job name"))?;

        let schedule_str = self.schedule.map(|s| {
            let _ = parse_cron(&s).expect("Invalid cron expression");
            s.to_string()
        });
        let schedule = schedule_str
            .or(parsed.schedule)
            .ok_or_else(|| Error::Cli("Missing job schedule"))?;

        let image_name = self
            .image_name
            .or(parsed.image_name)
            .ok_or_else(|| Error::Cli("Missing image name"))?;

        let queue_name = self
            .queue_name
            .or(parsed.queue_name)
            .ok_or_else(|| Error::Cli("Missing queue name"))?;

        let timeout_seconds = self
            .timeout_seconds
            .or(parsed.timeout_seconds)
            .unwrap_or(7200);

        let max_concurrency = self
            .max_concurrency
            .or(parsed.max_concurrency)
            .unwrap_or(100);

        let request = AddJobRequest {
            job_name,
            description: self.description.or(parsed.description),
            schedule,
            image_name,
            queue_name,
            max_concurrency,
            timeout_seconds,
            priority: self.priority.or(parsed.priority).unwrap_or(1),
            no_retry: self.no_retry.or(parsed.no_retry).unwrap_or(false),
            options: self.options.or(parsed.options),
            command: self.command.or(parsed.command),
            args: self.args.or(parsed.args).unwrap_or_default(),
        };

        Ok(request)
    }
}

#[derive(Debug, Args)]
#[clap(group(
     clap::ArgGroup::new("identifier")
         .required(true).args(&["id", "name"])
 ))]
pub struct JobIdentifierArgs {
    /// UUID of the job
    #[arg(long)]
    pub job_id: Option<Uuid>,

    /// Name of the job
    #[arg(long)]
    pub job_name: Option<String>,
}

impl Display for JobIdentifierArgs {
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

impl From<JobIdentifierArgs> for JobIdentifierRequest {
    fn from(value: JobIdentifierArgs) -> Self {
        match (value.job_id, value.job_name) {
            (Some(id), Some(_name)) => JobIdentifierRequest {
                job_identifier: Some(JobIdentifier::JobId(id.to_string())),
            },
            (Some(id), None) => JobIdentifierRequest {
                job_identifier: Some(JobIdentifier::JobId(id.to_string())),
            },
            (None, Some(name)) => JobIdentifierRequest {
                job_identifier: Some(JobIdentifier::JobName(name)),
            },
            (None, None) => JobIdentifierRequest {
                job_identifier: None,
            },
        }
    }
}

/// Wrapper struct to display a list of [`JobItem`] in a table with [tabled::Table].
#[derive(Tabled)]
struct JobDisplay {
    #[tabled(rename = "ID")]
    pub job_id: String,

    #[tabled(rename = "NAME")]
    pub job_name: String,

    #[tabled(rename = "SCHEDULE")]
    pub schedule: String,

    #[tabled(rename = "IMAGE")]
    pub image_name: String,

    #[tabled(rename = "QUEUE")]
    pub queue_name: String,

    #[tabled(rename = "PRIORITY")]
    pub priority: i32,

    #[tabled(rename = "RETRY")]
    pub no_retry: bool,

    #[tabled(rename = "NEXT OCCURRENCE")]
    next_occurrence_at: String,
}

impl From<JobItem> for JobDisplay {
    fn from(value: JobItem) -> Self {
        let deadline = value
            .next_occurrence_at
            .as_ref()
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Invalid Date".to_string())
            })
            .unwrap_or_else(|| "N/A".to_string());

        Self {
            job_id: value.job_id,
            job_name: value.job_name,
            schedule: value.schedule,
            image_name: value.image_name,
            queue_name: value.queue_name,
            priority: value.priority,
            no_retry: value.no_retry,
            next_occurrence_at: deadline,
        }
    }
}

/// Defines commands for managing jobs within Repeatrs.
///
/// This enum is used with `clap` to parse command-line arguments and provides a
/// structure for different job-related operations.
///
/// # Examples
///
/// ## Adding a new job (`add`)
/// ```bash
/// repeatrs job add \
///   --name "my-daily-backup" \
///   --description "Performs a daily backup of critical data" \
///   --schedule "0 0 * * *" \
///   --repository "my-registry/backup-image" \
///   --tag "latest" \
///   --queue-name "high-priority" \
///   --command "python main.py" \
///   --args "--target /data --compress gzip" \
///   --priority 10 \
///   --max-concurrent-runs 1 \
///   --timeout-seconds 3600 \
///   --no-retry
/// ```
///
/// ## Adding a job with dry run (`add --dry-run`)
/// ```bash
/// repeatrs job add \
///   --name "test-job" \
///   --schedule "* * * * *" \
///   --repository "nginx" \
///   --tag "latest" \
///   --dry-run
/// ```
///
/// ## Removing a job by UUID (`remove --id`)
/// ```bash
/// repeatrs job remove --id 123e4567-e89b-12d3-a456-426614174000
/// ```
///
/// ## Removing a job by name (`remove --name`)
/// ```bash
/// repeatrs job remove --name "my-daily-backup"
/// ```
///
/// ## Deactivating a job by UUID (`deactivate --id`)
/// ```bash
/// repeatrs job deactivate --id 123e4567-e89b-12d3-a456-426614174000
/// ```
///
/// ## Deactivating a job by name (`deactivate --name`)
/// ```bash
/// repeatrs job deactivate --name "my-daily-backup"
/// ```
///
/// ## Showing job information by UUID (`show --id`)
/// ```bash
/// repeatrs job show --id 123e4567-e89b-12d3-a456-426614174000
/// ```
///
/// ## Showing job information by name (`show --name`)
/// ```bash
/// repeatrs job show --name "my-daily-backup"
/// ```
///
/// ## Listing all jobs (`list`)
/// ```bash
/// repeatrs job list
/// ```
///
/// ## Showing logs for a job by UUID (`logs --id`)
/// ```bash
/// repeatrs job logs --id 123e4567-e89b-12d3-a456-426614174000
/// ```
///
/// ## Showing logs for a job by name (`logs --name`)
/// ```bash
/// repeatrs job logs --name "my-daily-backup"
/// ```
#[derive(Debug, Subcommand)]
pub enum JobSubcommand {
    // Add a newjob
    #[command(about = "add a new job")]
    Add {
        #[command(flatten)]
        args: AddJobArgs,
    },

    /// Remove a job by its name or UUID
    #[command(about = "delete a job by its name or UUID")]
    Delete {
        #[command(flatten)]
        identifier: JobIdentifierArgs,
    },

    /// Deactivate a job by its name or UUID
    #[command(about = "deactivate a job by its name or UUID")]
    Deactivate {
        #[command(flatten)]
        identifier: JobIdentifierArgs,
    },

    // Show job informations by name or UUID
    #[command(about = "show job information by name or UUID")]
    Show {
        #[command(flatten)]
        identifier: JobIdentifierArgs,
    },

    // Show the list of jobs
    #[command(alias = "ls", about = "show the list of jobs")]
    List {
        #[command(flatten)]
        identifier: QueueIdentifierArgs,
    },

    // Show the last log for a specific job
    #[command(about = "show the last log for a job given its name or UUID")]
    Logs {
        #[command(flatten)]
        identifier: JobIdentifierArgs,
    },
}

impl Display for JobSubcommand {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Add { args } => write!(f, "add job:\n{}", args),
            Self::Deactivate { identifier } => write!(f, "deactivate job: {}", identifier),
            Self::Delete { identifier } => write!(f, "delete job: {}", identifier),
            Self::Show { identifier } => write!(f, "show job: {}", identifier),
            Self::List { identifier: _ } => write!(f, "list jobs"),
            Self::Logs { identifier } => write!(f, "logs for job: {}", identifier),
        }
    }
}

pub struct JobService {
    client: GrpcJobServiceClient<Channel>,
}

impl JobService {
    pub fn with_channel(channel: Channel) -> Self {
        let client = GrpcJobServiceClient::new(channel);
        Self { client }
    }

    pub async fn execute(self, command: JobSubcommand) -> Result<()> {
        match command {
            JobSubcommand::Add { args } => self.add_job(args).await,
            JobSubcommand::Deactivate { identifier } => self.deactivate_job(identifier).await,
            JobSubcommand::Delete { identifier } => self.delete_job(identifier).await,
            JobSubcommand::Show { identifier: _ } => todo!(),
            JobSubcommand::Logs { identifier: _ } => todo!(),
            JobSubcommand::List { identifier } => self.list_jobs(identifier).await,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn list_jobs(mut self, identifier: QueueIdentifierArgs) -> Result<()> {
        let request: QueueIdentifierRequest = identifier.into();

        let response = self.client.list_jobs(request).await?;

        let inner = response.into_inner();

        let display_data: Vec<JobDisplay> = inner.jobs.into_iter().map(JobDisplay::from).collect();

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(dry_run = args.dry_run))]
    async fn add_job(self, args: AddJobArgs) -> Result<()> {
        if args.dry_run {
            println!("{}", args);
            Ok(())
        } else {
            let payload = args.resolve()?;
            self.add_job_inner(payload).await
        }
    }

    #[tracing::instrument(skip_all)]
    async fn add_job_inner(mut self, payload: AddJobRequest) -> Result<()> {
        let name = payload.job_name.clone();
        let response: Response<JobServiceResponse> =
            self.client.add_job(Request::new(payload)).await?;
        let inner = response.get_ref();

        if inner.success {
            println!("{}", inner.job_id);
        } else {
            println!("Failed creating job '{}'", name);
        }
        Ok(())
    }

    pub async fn deactivate_job(mut self, identifier: JobIdentifierArgs) -> Result<()> {
        let request: JobIdentifierRequest = identifier.into();

        let response = self.client.deactivate_job(request).await?;

        let inner = response.get_ref();

        if inner.success {
            println!("{}", inner.job_id)
        } else {
            println!("Failed deactivating job")
        }

        Ok(())
    }

    pub async fn delete_job(mut self, identifier: JobIdentifierArgs) -> Result<()> {
        let request: JobIdentifierRequest = identifier.into();

        let response = self.client.delete_job(request).await?;

        let inner = response.get_ref();

        if inner.success {
            println!("{}", inner.job_id);
        } else {
            println!("Failed deleting job");
        }

        Ok(())
    }
}
