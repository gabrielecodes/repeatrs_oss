mod queries;

use queries::*;

use crate::{DbResult, error::DbError};

use chrono::{DateTime, Utc};
use repeatrs_domain::{JobScheduleState, JobScheduleStateOperations};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgJobSchedulerStateRepository;

impl<'e> JobScheduleStateOperations<Transaction<'e, Postgres>> for PgJobSchedulerStateRepository {
    type Err = DbError;

    /// Insert multiple job schedules
    async fn upsert_jobs_schedule(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_schedules: &[JobScheduleState],
    ) -> DbResult<()> {
        let db_job_schedules: Vec<DbJobScheduleState> =
            job_schedules.into_iter().map(|s| s.into()).collect();

        let _ = upsert_jobs_schedule(&mut **tx, &db_job_schedules).await?;

        Ok(())
    }
}

pub struct DbJobScheduleState {
    job_id: Uuid,
    next_run_at: DateTime<Utc>,
    last_scheduled_at: Option<DateTime<Utc>>,
}

impl From<JobScheduleState> for DbJobScheduleState {
    fn from(value: JobScheduleState) -> Self {
        DbJobScheduleState {
            job_id: value.job_id_inner(),
            next_run_at: value.next_run_at().clone(),
            last_scheduled_at: value.last_scheduled_at().clone(),
        }
    }
}

impl From<&JobScheduleState> for DbJobScheduleState {
    fn from(value: &JobScheduleState) -> Self {
        DbJobScheduleState {
            job_id: value.job_id_inner(),
            next_run_at: value.next_run_at().clone(),
            last_scheduled_at: value.last_scheduled_at().clone(),
        }
    }
}
