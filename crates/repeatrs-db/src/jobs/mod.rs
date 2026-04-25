//! This module contains the job model, associated convenience types, getters
//! and an implementation of the [`Repository`] trait for the [`Job`] type.
//! Refer to [`queries`] for CRUD operations and queries on the `jobs` table.

mod queries;

use queries::*;

use crate::{DbResult, error::DbError, queues::DbQueueId};

use chrono::{DateTime, Utc};
use croner::Cron;
use repeatrs_domain::{Job, JobId, JobOperations, JobResponse, JobStatus, QueueId};
use sqlx::{Postgres, Transaction, Type};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone)]
pub struct PgJobRepository;

impl<'e> JobOperations<Transaction<'e, Postgres>> for PgJobRepository {
    type Err = DbError;

    async fn add_job(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job: &Job,
        queue_id: &QueueId,
    ) -> DbResult<JobId> {
        let job_id = add_job(&mut **tx, job, &queue_id.into()).await?;

        Ok(job_id.into())
    }

    /// Returns true if the job with the given id already exists.
    async fn get_job_by_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_id: &JobId,
    ) -> DbResult<JobResponse> {
        let job_row = get_job_by_id(&mut **tx, &job_id.into()).await?;

        let job: JobResponse = JobResponse::try_from(job_row)?;

        Ok(job)
    }

    /// Returns true if the job with the given name already exists.
    async fn get_job_by_name(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_name: &str,
    ) -> DbResult<JobResponse> {
        let job_row = get_job_by_name(&mut **tx, job_name).await?;

        let job: JobResponse = JobResponse::try_from(job_row)?;

        Ok(job)
    }

    async fn get_jobs_by_queue_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_id: &QueueId,
    ) -> DbResult<Vec<JobResponse>> {
        let job_rows = get_jobs_by_queue_id(&mut **tx, &queue_id.into()).await?;
        let jobs: DbResult<Vec<JobResponse>> = job_rows.into_iter().map(|j| j.try_into()).collect();

        jobs
    }

    async fn get_jobs_by_queue_name(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_name: &str,
    ) -> DbResult<Vec<JobResponse>> {
        let jobs = get_jobs_by_queue_name(&mut **tx, queue_name).await?;

        let jobs: DbResult<Vec<JobResponse>> = jobs.into_iter().map(|j| j.try_into()).collect();

        jobs
    }

    /// Sets the job to [`DbJobStatus::Inactive`].
    async fn deactivate_job_by_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_id: &JobId,
    ) -> DbResult<JobId> {
        let job = get_job_by_id(&mut **tx, &job_id.into()).await?;

        let job_id = job.deactivate_job_by_id(&mut **tx).await?;

        Ok(job_id.into())
    }

    /// Sets the job to [`DbJobStatus::Inactive`].
    async fn deactivate_job_by_name(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_name: &str,
    ) -> DbResult<JobId> {
        let job = get_job_by_name(&mut **tx, job_name).await?;

        let job_id = job.deactivate_job_by_name(&mut **tx).await?;

        Ok(job_id.into())
    }

    async fn delete_job_by_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_id: &JobId,
    ) -> DbResult<JobId> {
        let job = get_job_by_id(&mut **tx, &job_id.into()).await?;

        let job_id = job.delete_job_by_id(&mut **tx).await?;

        Ok(job_id.into())
    }

    async fn delete_job_by_name(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_name: &str,
    ) -> DbResult<JobId> {
        let job = get_job_by_name(&mut **tx, job_name).await?;

        let job_id = job.delete_job_by_name(&mut **tx).await?;

        Ok(job_id.into())
    }

    // async fn get_earliest_deadline(&self, tx: &mut Transaction<'e, Postgres>) -> DbResult<Instant> {
    //     Ok(get_earliest_deadline(&mut **tx).await)
    // }

    async fn get_due_jobs(&self, tx: &mut Transaction<'e, Postgres>) -> DbResult<Vec<JobResponse>> {
        let jobs = get_due_jobs(&mut **tx).await?;

        let jobs: DbResult<Vec<JobResponse>> = jobs.into_iter().map(|j| j.try_into()).collect();

        jobs
    }

    // async fn update_deadlines(
    //     &self,
    //     tx: &mut Transaction<'e, Postgres>,
    //     jobs: &[Job],
    // ) -> DbResult<()> {
    //     let job_rows: Vec<JobRow> = jobs.iter().map(|j| j.into()).collect();

    //     update_deadlines(&mut **tx, &job_rows).await?;

    //     Ok(())
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(transparent)]
pub struct DbJobId(Uuid);

impl From<DbJobId> for JobId {
    fn from(value: DbJobId) -> Self {
        JobId::new(value.0)
    }
}

impl From<JobId> for DbJobId {
    fn from(value: JobId) -> Self {
        DbJobId(value.inner())
    }
}

impl From<&JobId> for DbJobId {
    fn from(value: &JobId) -> Self {
        DbJobId(value.inner())
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Type)]
#[sqlx(type_name = "job_status", rename_all = "UPPERCASE")]
pub enum DbJobStatus {
    /// Job can be scheduled for execution
    #[default]
    Active,

    /// Job is not going to be scheduled for execution    
    Inactive,
}

impl From<DbJobStatus> for JobStatus {
    fn from(value: DbJobStatus) -> Self {
        match value {
            DbJobStatus::Active => JobStatus::Active,
            DbJobStatus::Inactive => JobStatus::Inactive,
        }
    }
}

impl From<JobStatus> for DbJobStatus {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Active => DbJobStatus::Active,
            JobStatus::Inactive => DbJobStatus::Inactive,
        }
    }
}

impl From<&JobStatus> for DbJobStatus {
    fn from(value: &JobStatus) -> Self {
        match value {
            JobStatus::Active => DbJobStatus::Active,
            JobStatus::Inactive => DbJobStatus::Inactive,
        }
    }
}

#[derive(Debug)]
struct JobRow {
    /// Unique ID of the job, primary key
    job_id: DbJobId,

    /// Unique job name
    job_name: String,

    /// The job description
    description: Option<String>,

    /// schedule of the cronjob or execution time
    schedule: String,

    /// Options for running the container
    options: Option<String>,

    /// Image name as <optional_registry>/<image_name>:tag
    image_name: String,

    /// The command to run the container
    command: Option<String>,

    /// Arguments for the command
    args: Option<String>,

    /// Retry the job if last execution failed
    max_retries: i32,

    /// Current status of the job
    status: DbJobStatus,

    /// Job priority
    priority: i32,

    /// Identifier of the queue this job belongs to
    queue_id: DbQueueId,

    /// Name of the queue this job belongs to
    queue_name: String,

    /// Maximum number of identical jobs running concurrently
    max_concurrency: i32,

    /// A hard limit on the duration of the job, after which the job is terminated.
    timeout_seconds: Option<i32>,

    /// Job creation timestamp
    created_at: DateTime<Utc>,

    updated_at: DateTime<Utc>,
}

impl JobRow {
    pub fn job_id(&self) -> DbJobId {
        self.job_id
    }

    pub fn job_name(&self) -> String {
        self.job_name.to_string()
    }
}

impl TryFrom<JobRow> for JobResponse {
    type Error = DbError;

    fn try_from(value: JobRow) -> Result<Self, Self::Error> {
        let schedule = Cron::from_str(&value.schedule)?;

        let timeout_seconds = value
            .timeout_seconds
            .map(|t| chrono::Duration::seconds(t as i64));

        let job = JobResponse::from_row(
            JobId::new(value.job_id().0),
            value.job_name,
            value.description,
            schedule,
            value.options,
            value.image_name,
            value.command,
            value.args,
            value.max_retries,
            value.status.into(),
            value.priority,
            value.queue_id.into(),
            value.queue_name,
            value.max_concurrency,
            timeout_seconds,
            value.created_at,
            value.updated_at,
        );

        Ok(job)
    }
}

impl From<&JobResponse> for JobRow {
    fn from(value: &JobResponse) -> Self {
        JobRow {
            job_id: DbJobId(value.job_id().inner()),
            job_name: value.job_name().to_owned(),
            description: value.description().map(ToString::to_string),
            schedule: value.schedule().to_string(),
            options: value.image_options().map(ToString::to_string),
            image_name: value.image_name().to_owned(),
            command: value.command().map(ToString::to_string),
            args: value.command_args().map(ToString::to_string),
            max_retries: value.max_retries(),
            status: value.status().into(),
            priority: value.priority(),
            queue_id: value.queue_id().into(),
            max_concurrency: value.max_concurrency(),
            timeout_seconds: value
                .timeout_seconds()
                .map(|d| d.num_seconds().clamp(i32::MIN as i64, i32::MAX as i64) as i32),
            created_at: value.created_at(),
            updated_at: value.updated_at(),
            queue_name: value.queue_name().to_string(),
        }
    }
}

impl From<JobResponse> for JobRow {
    fn from(value: JobResponse) -> JobRow {
        JobRow::from(&value)
    }
}
