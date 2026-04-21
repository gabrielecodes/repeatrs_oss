use crate::error::DomainError;
use crate::{IsId, QueueId};

use chrono::{DateTime, Utc};
use croner::Cron;
use repeatrs_proto::repeatrs::{AddJobRequest, JobItem};
use serde::Serialize;
use std::{fmt::Display, ops::Deref, str::FromStr};
use tokio::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobId(Uuid);
impl IsId for JobId {}

impl Deref for JobId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for JobId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<JobId> for Uuid {
    fn from(job_id: JobId) -> Self {
        job_id.0
    }
}

impl FromStr for JobId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(JobId(Uuid::from_str(s)?))
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl JobId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn inner(self) -> Uuid {
        self.0
    }
}

/// Describes the status of the job.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize)]
pub enum JobStatus {
    /// Job can be scheduled for execution
    #[default]
    Active,

    /// Job is not going to be scheduled for execution    
    Inactive,
}

impl Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Active => "ACTIVE",
            Self::Inactive => "INACTIVE",
        };
        f.write_str(s)
    }
}

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ACTIVE" => Ok(Self::Active),
            "INACTIVE" => Ok(Self::Inactive),
            _ => Err("Job status not understood.".into()),
        }
    }
}

impl From<JobStatus> for String {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Active => "ACTIVE".to_string(),
            JobStatus::Inactive => "INACTIVE".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Job {
    /// Unique ID of the job, primary key
    pub job_id: JobId,

    /// Unique job name
    pub job_name: String,

    /// The job description
    pub description: Option<String>,

    /// schedule of the cronjob or execution time
    pub schedule: Cron,

    /// Options for running the container
    pub options: Option<String>,

    /// Image name as <optional_registry>/<image_name>:tag
    pub image_name: String,

    /// The command to run the container
    pub command: Option<String>,

    /// Arguments for the command
    pub args: Option<String>,

    /// Retry the job if last execution failed
    pub max_retries: i32,

    /// Current status of the job
    pub status: JobStatus,

    /// Job priority
    pub priority: Option<i32>,

    /// Identifier of the queue this job belongs to
    pub queue_id: QueueId,

    /// Name of the queue this job belongs to
    pub queue_name: String,

    /// Maximum number of identical jobs running concurrently
    pub max_concurrency: Option<i32>,

    /// A hard limit on the duration of the job, after which the job is terminated. Default: 2 hours
    pub timeout_seconds: Option<i32>,

    /// Job creation timestamp
    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

impl Job {
    pub fn to_job_item(&self, queue_name: &str) -> JobItem {
        JobItem {
            job_id: self.job_id.inner().into(),
            job_name: self.job_name.to_string(),
            status: self.status.to_string(),
            schedule: self.schedule.to_string(),
            image_name: self.image_name.to_string(),
            queue_name: queue_name.to_string(),
            max_retries: self.max_retries,
            priority: self.priority.unwrap_or_default(),
        }
    }
}

/// Data Transfer Object to translate gRPC (prost) types to an input usable
/// at the service layer
#[derive(Clone)]
pub struct JobDefinition {
    // Mandatory fields
    pub job_name: String,
    pub schedule: String,
    pub image_name: String,
    pub queue_name: String,
    // Fields with defaults
    pub max_retries: i32,
    pub priority: i32,
    pub max_concurrency: i32,
    pub timeout_seconds: i32,
    // Non mandatory fields without defaults
    pub description: Option<String>,
    pub options: Option<String>,
    pub command: Option<String>,
    pub args: Option<String>,
}

impl TryFrom<AddJobRequest> for JobDefinition {
    type Error = DomainError;

    //TODO: missing checks
    fn try_from(value: AddJobRequest) -> core::result::Result<Self, Self::Error> {
        if let Err(e) = value.schedule.parse::<Cron>() {
            return Err(Self::Error::Validation(e.to_string()));
        };

        let job_name = value.job_name.replace(" ", "_");
        let queue_name = value.queue_name.replace(" ", "_");

        let args = if value.args.is_empty() {
            None::<String>
        } else {
            Some(value.args.join(" "))
        };

        let def = JobDefinition {
            job_name,
            queue_name,
            description: value.description,
            schedule: value.schedule,
            image_name: value.image_name,
            max_retries: value.max_retries,
            options: value.options,
            command: value.command,
            args,
            priority: value.priority,
            max_concurrency: value.max_concurrency,
            timeout_seconds: value.timeout_seconds,
        };

        Ok(def)
    }
}

pub trait JobOperations<E>: Sync + Send + 'static {
    type Err: std::error::Error;

    fn add_job(
        &self,
        tx: &mut E,
        job: &JobDefinition,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    /// Returns true if the job with the given id already exists.
    fn get_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<Job, Self::Err>> + Send;

    /// Returns true if the job with the given name already exists.
    fn get_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<Job, Self::Err>> + Send;

    fn get_jobs_by_queue_id(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<Vec<Job>, Self::Err>> + Send;

    fn get_jobs_by_queue_name(
        &self,
        tx: &mut E,
        queue_name: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Job>, Self::Err>> + Send;

    fn deactivate_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn deactivate_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn delete_job_by_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    fn delete_job_by_name(
        &self,
        tx: &mut E,
        job_name: &str,
    ) -> impl std::future::Future<Output = Result<JobId, Self::Err>> + Send;

    // fn get_earliest_deadline(
    //     &self,
    //     tx: &mut E,
    // ) -> impl std::future::Future<Output = Result<Instant, Self::Err>> + Send;

    fn get_due_jobs(
        &self,
        tx: &mut E,
    ) -> impl std::future::Future<Output = Result<Vec<Job>, Self::Err>> + Send;

    // fn update_deadlines(
    //     &self,
    //     tx: &mut E,
    //     jobs: &[Job],
    // ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;
}
