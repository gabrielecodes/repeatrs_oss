use crate::JobId;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct NewJobScheduleState {
    job_id: JobId,
    next_run_at: DateTime<Utc>,
}

impl NewJobScheduleState {
    pub fn new(job_id: &JobId, next_run_at: DateTime<Utc>) -> Self {
        Self {
            job_id: job_id.to_owned(),
            next_run_at,
        }
    }

    pub fn job_id_inner(&self) -> Uuid {
        self.job_id.inner()
    }

    pub fn next_run_at(&self) -> DateTime<Utc> {
        self.next_run_at
    }
}

#[derive(Debug)]
pub struct JobScheduleState {
    job_id: JobId,
    next_run_at: DateTime<Utc>,
    last_scheduled_at: Option<DateTime<Utc>>,
}

impl JobScheduleState {
    pub fn new(job_id: JobId, next_run_at: DateTime<Utc>) -> Self {
        Self {
            job_id,
            next_run_at,
            last_scheduled_at: None,
        }
    }

    pub fn job_id_inner(&self) -> Uuid {
        self.job_id.inner()
    }

    pub fn next_run_at(&self) -> &DateTime<Utc> {
        &self.next_run_at
    }

    pub fn last_scheduled_at(&self) -> Option<DateTime<Utc>> {
        self.last_scheduled_at
    }
}

pub trait JobScheduleStateOperations<E>: Sync + Send + 'static {
    type Err: std::error::Error;

    /// Inserts multiple job schedules
    fn upsert_jobs_schedule(
        &self,
        tx: &mut E,
        job_schedules: &[NewJobScheduleState],
    ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;
}
