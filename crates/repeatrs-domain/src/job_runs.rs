use crate::{IsId, JobId, QueueId, WorkerId};

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{fmt::Display, ops::Deref, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobRunId(Uuid);
impl IsId for JobRunId {}

impl Deref for JobRunId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for JobRunId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<JobRunId> for Uuid {
    fn from(job_id: JobRunId) -> Self {
        job_id.0
    }
}

impl FromStr for JobRunId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(JobRunId(Uuid::from_str(s)?))
    }
}

impl std::fmt::Display for JobRunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl JobRunId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn inner(self) -> Uuid {
        self.0
    }
}

/// Describes the status of the job.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize)]
pub enum JobRunStatus {
    /// The job is queued and waiting capacity
    #[default]
    Queued,

    /// The job is currently being executed
    Running,

    /// The job was stopped with SIGTERM/SIGKILL
    Stopped,

    /// The job exited with a non-zero exit status
    Failed,

    /// The job run is not executed
    Skipped,

    /// The job exited with a zero exit status
    Completed,
}

impl Display for JobRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "ACTIVE",
            Self::Running => "RUNNING",
            Self::Stopped => "STOPPED",
            Self::Failed => "FAILED",
            Self::Skipped => "SKIPPED",
            Self::Completed => "COMPLETED",
        };
        f.write_str(s)
    }
}

impl FromStr for JobRunStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "QUEUED" => Ok(Self::Queued),
            "RUNNING" => Ok(Self::Running),
            "STOPPED" => Ok(Self::Stopped),
            "FAILED" => Ok(Self::Failed),
            "SKIPPED" => Ok(Self::Skipped),
            "COMPLETED" => Ok(Self::Completed),
            _ => Err("Job status not understood.".into()),
        }
    }
}

impl From<JobRunStatus> for String {
    fn from(value: JobRunStatus) -> Self {
        match value {
            JobRunStatus::Queued => "QUEUED".to_string(),
            JobRunStatus::Running => "RUNNING".to_string(),
            JobRunStatus::Stopped => "STOPPED".to_string(),
            JobRunStatus::Failed => "FAILED".to_string(),
            JobRunStatus::Skipped => "SKIPPED".to_string(),
            JobRunStatus::Completed => "COMPLETED".to_string(),
        }
    }
}

// domain/src/job.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    PurposelyStopped,     // 0
    ApplicationError,     // 1
    ContainerFailedToRun, // 125
    CommandInvokeError,   // 126
    FileNotFound,         // 127
    InvalidExitArgument,  // 128
    AbnormalTermination,  // 134 (SIGABRT)
    ImmediateTermination, // 137 (SIGKILL)
    SegmentationFault,    // 139 (SIGSEGV)
    GracefulTermination,  // 143 (SIGTERM)
    OutOfRange,           // 255
    Unknown(i32),         // Catch-all
}

impl From<i32> for ExitStatus {
    fn from(code: i32) -> Self {
        match code {
            0 => Self::PurposelyStopped,
            1 => Self::ApplicationError,
            125 => Self::ContainerFailedToRun,
            126 => Self::CommandInvokeError,
            127 => Self::FileNotFound,
            128 => Self::InvalidExitArgument,
            134 => Self::AbnormalTermination,
            137 => Self::ImmediateTermination,
            139 => Self::SegmentationFault,
            143 => Self::GracefulTermination,
            255 => Self::OutOfRange,
            other => Self::Unknown(other),
        }
    }
}

impl From<ExitStatus> for i32 {
    fn from(status: ExitStatus) -> Self {
        match status {
            ExitStatus::PurposelyStopped => 0,
            ExitStatus::ApplicationError => 1,
            ExitStatus::ContainerFailedToRun => 125,
            ExitStatus::CommandInvokeError => 126,
            ExitStatus::FileNotFound => 127,
            ExitStatus::InvalidExitArgument => 128,
            ExitStatus::AbnormalTermination => 134,
            ExitStatus::ImmediateTermination => 137,
            ExitStatus::SegmentationFault => 139,
            ExitStatus::GracefulTermination => 143,
            ExitStatus::OutOfRange => 255,
            ExitStatus::Unknown(code) => code,
        }
    }
}

/// Represents the information necessary to insert a new job run in the database
#[derive(Debug)]
pub struct NewJobRun {
    /// The unique identifier of the job executed in this job run
    job_id: JobId,

    /// The unique identifier of the queue this job belongs to
    queue_id: QueueId,

    /// Scheduled execution time
    scheduled_time: DateTime<Utc>,
}

impl NewJobRun {
    pub fn new(job_id: &JobId, queue_id: &QueueId, scheduled_time: &DateTime<Utc>) -> Self {
        Self {
            job_id: job_id.to_owned(),
            queue_id: queue_id.to_owned(),
            scheduled_time: scheduled_time.to_owned(),
        }
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn queue_id(&self) -> &QueueId {
        &self.queue_id
    }

    pub fn scheduled_time(&self) -> DateTime<Utc> {
        self.scheduled_time
    }
}

/// Represents a job run that has been persisted in the database.
#[derive(Debug)]
pub struct JobRun {
    /// Unique ID of the job run, primary key
    job_run_id: JobRunId,

    /// The unique identifier of the job executed in this job run
    job_id: JobId,

    /// The unique identifier of the queue this job belongs to
    queue_id: QueueId,

    /// The unique identifier of the worker that claimed this job
    worker_id: Option<WorkerId>,

    /// Time when a worker claimed this job run
    claimed_at: Option<DateTime<Utc>>,

    /// Current status of the job
    status: JobRunStatus,

    /// Scheduled execution time
    scheduled_time: DateTime<Utc>,

    /// Number of times this job has been attempted
    attempt_count: i32,

    /// Job start time
    started_at: Option<DateTime<Utc>>,

    /// Job end time
    ended_at: Option<DateTime<Utc>>,

    /// Name of the queue this job belongs to
    exit_code: Option<ExitStatus>,

    /// Maximum number of identical jobs running concurrently
    error: Option<String>,

    /// Job creation timestamp
    created_at: DateTime<Utc>,

    /// Time this job run was last updated
    updated_at: DateTime<Utc>,
}

impl JobRun {
    // NOTE: we want to keep the fields of JobRun private and avoid
    // defining another DTO with public fields. With Builders
    // it's possible to partially instantiated objects. JobRun and
    // JobRunRow are internal types not exposed publicly so the
    // inconvenience of having too many fields does not leak.
    /// Builds a [`JobRun`] from database informations
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        job_run_id: JobRunId,
        job_id: JobId,
        queue_id: QueueId,
        worker_id: Option<WorkerId>,
        claimed_at: Option<DateTime<Utc>>,
        status: JobRunStatus,
        scheduled_time: DateTime<Utc>,
        attempt_count: i32,
        started_at: Option<DateTime<Utc>>,
        ended_at: Option<DateTime<Utc>>,
        exit_code: Option<ExitStatus>,
        error: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> JobRun {
        JobRun {
            job_run_id,
            job_id,
            queue_id,
            worker_id,
            claimed_at,
            status,
            scheduled_time,
            attempt_count,
            started_at,
            ended_at,
            exit_code,
            error,
            created_at,
            updated_at,
        }
    }

    pub fn job_run_id(&self) -> &JobRunId {
        &self.job_run_id
    }

    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    pub fn queue_id(&self) -> &QueueId {
        &self.queue_id
    }

    pub fn worker_id(&self) -> Option<WorkerId> {
        self.worker_id
    }

    pub fn claimed_at(&self) -> Option<DateTime<Utc>> {
        self.claimed_at
    }

    pub fn status(&self) -> &JobRunStatus {
        &self.status
    }

    pub fn scheduled_time(&self) -> &DateTime<Utc> {
        &self.scheduled_time
    }

    pub fn attempt_count(&self) -> i32 {
        self.attempt_count
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    pub fn exit_code(&self) -> Option<ExitStatus> {
        self.exit_code
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }
}

pub trait JobRunOperations<E>: Sync + Send + 'static {
    type Err: std::error::Error;

    /// Inserts multiple job runs
    fn insert_job_runs(
        &self,
        tx: &mut E,
        job_run_info: &[NewJobRun],
    ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;

    /// Returns all the job runs of a specific job given its id
    fn get_job_runs_by_job_id(
        &self,
        tx: &mut E,
        job_id: &JobId,
    ) -> impl std::future::Future<Output = Result<Vec<JobRun>, Self::Err>> + Send;

    /// Returns all the job runs of jobs belonging to a specific queue
    fn get_job_runs_by_queue_id(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<Vec<JobRun>, Self::Err>> + Send;

    /// Returns the job runs
    fn get_latest_job_runs(
        &self,
        tx: &mut E,
    ) -> impl std::future::Future<Output = Result<Vec<JobRun>, Self::Err>> + Send;
}
