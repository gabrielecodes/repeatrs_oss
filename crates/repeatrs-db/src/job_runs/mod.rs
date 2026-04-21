mod queries;

use queries::*;

use crate::{DbResult, error::DbError, jobs::DbJobId, queues::DbQueueId, workers::DbWorkerId};

use chrono::{DateTime, Utc};
use repeatrs_domain::{JobId, JobRun, JobRunId, JobRunInsert, JobRunOperations, JobRunStatus};
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
        job_info: &[JobRunInsert],
    ) -> DbResult<()> {
        let _ = insert_job_runs(&mut **tx, job_info).await?;

        Ok(())
    }

    /// Returns the job run given its id
    async fn get_job_run_by_job_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_id: &JobId,
    ) -> DbResult<Vec<JobRun>> {
        let db_job_runs = get_job_run_by_job_id(&mut **tx, job_id.into()).await?;

        let job_runs: Vec<JobRun> = db_job_runs.into_iter().map(|r| r.into()).collect();

        Ok(job_runs)
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

impl From<JobRunId> for DbJobRunId {
    fn from(value: JobRunId) -> Self {
        DbJobRunId(value.inner())
    }
}

impl From<&JobRunId> for DbJobRunId {
    fn from(value: &JobRunId) -> Self {
        DbJobRunId(value.inner())
    }
}

impl From<DbJobRunId> for JobRunId {
    fn from(value: DbJobRunId) -> Self {
        JobRunId::new(value.0)
    }
}

/// A record of a [`Job`] execution.
pub struct JobRunRow {
    /// Unique identifier for a job run
    job_run_id: DbJobRunId,

    /// The job this job run refers to.
    job_id: DbJobId,

    /// The job this job run refers to.
    queue_id: DbQueueId,

    /// Identifier of the worker that concluded the execution.
    worker_id: DbWorkerId,

    /// Time when a worker claimed this job run
    claimed_at: Option<DateTime<Utc>>,

    /// Current status of the container.
    status: DbJobRunStatus,

    /// Scheduled execution time
    scheduled_time: DateTime<Utc>,

    /// Number of times this job has been attempted
    attempt_count: i32,

    /// Timestamp representing the start time of this run
    started_at: Option<DateTime<Utc>>,

    /// Timestamp representing the end time of this run
    ended_at: Option<DateTime<Utc>>,    

    /// Container exit code
    exit_code: Option<i32>,

    /// Error message
    error: Option<String>,

    /// Job creation timestamp
    created_at: DateTime<Utc>,

    /// Time this job run was last updated
    updated_at: DateTime<Utc>,
}

impl From<JobRun> for JobRunRow {
    fn from(value: JobRun) -> Self {


        JobRunRow {
            job_run_id: value.job_run_id().into(),
            job_id: value.job_id().into(),
            queue_id: value.queue_id().into(),
            worker_id: value.worker_id().into(),
            claimed_at: value.claimed_at(),
            status: value.status().into(),
            scheduled_time: value.scheduled_time(),
            attempt_count: value.attempt_count(),
            started_at: value.started_at(),
            ended_at: value.,
            exit_code: value.,
            error: value.,
            created_at: value.,
            updated_at: value.,
        }
    }
}
