mod queries;

use queries::*;

use crate::{DbResult, error::DbError, workers::DbWorkerId};

use chrono::{DateTime, Utc};
use repeatrs_domain::{Job, JobRunId, JobRunOperations, JobRunStatus};
use sqlx::{Postgres, Transaction};
use std::{fmt::Display, ops::Deref, str::FromStr};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgJobRunRepository;

impl<'e> JobRunOperations<Transaction<'e, Postgres>> for PgJobRunRepository {
    type Err = DbError;

    /// Inserts multiple job runs
    async fn add_job_runs(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job: &Job,
    ) -> DbResult<JobRunId> {
        Ok(job_id.into())
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq, sqlx::Type)]
#[sqlx(type_name = "job_run_status", rename_all = "UPPERCASE")]
pub enum DbJobRunStatus {
    /// The job is queued and waiting capacity
    #[default]
    Queued,

    /// The job is currently being executed
    Running,

    /// The job was stopped with SIGTERM/SIGKILL
    Stopped,

    /// The job exited with a non-zero exit status
    Failed,

    /// The job exited with a zero exit status
    Completed,
}

impl From<JobRunStatus> for DbJobRunStatus {
    fn from(value: JobRunStatus) -> Self {
        match value {
            JobRunStatus::Queued => DbJobRunStatus::Queued,
            JobRunStatus::Running => DbJobRunStatus::Running,
            JobRunStatus::Stopped => DbJobRunStatus::Stopped,
            JobRunStatus::Failed => DbJobRunStatus::Failed,
            JobRunStatus::Completed => DbJobRunStatus::Completed,
        }
    }
}

impl Display for DbJobRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Queued => "QUEUED",
            Self::Running => "RUNING",
            Self::Stopped => "STOPPED",
            Self::Failed => "FAILED",
            Self::Completed => "COMPLETED",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for DbJobRunStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(Self::Queued),
            "RUNNING" => Ok(Self::Running),
            "STOPPED" => Ok(Self::Stopped),
            "FAILED" => Ok(Self::Failed),
            "COMPLETED" => Ok(Self::Completed),
            &_ => Err("Job Run Status not understood".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct DbJobRunId(pub Uuid);

impl Deref for DbJobRunId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for DbJobRunId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

/// A record of a [`Job`] execution.
#[derive(sqlx::FromRow, sqlx::Type)]
#[sqlx(type_name = "job_runs")]
pub struct JobRun {
    /// Unique identifier for a job run
    job_run_id: DbJobRunId,

    /// The job this job run refers to.
    job_id: Uuid,

    /// Identifier of the worker that concluded the execution.
    worker_id: DbWorkerId,

    /// Identifier of the container spawned to execute this job.
    container_name: String,

    /// Current status of the container.
    status: DbJobRunStatus,

    /// Timestamp representing the start time of this run
    started_at: Option<DateTime<Utc>>,

    /// Timestamp representing the end time of this run
    ended_at: Option<DateTime<Utc>>,

    /// Duration in seconds for this job run
    duration_secs: Option<i32>,

    /// Container exit code
    exit_code: Option<i32>,

    /// Error message
    error: Option<String>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl JobRun {
    pub fn job_run_id(&self) -> JobRunId {
        self.job_run_id.into()
    }

    // pub fn container_id(&self) -> String {
    //     self.container_id.to_owned()
    // }
}
