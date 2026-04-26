mod queries;

use queries::*;

use crate::{DbResult, error::DbError};

use chrono::{DateTime, Utc};
use repeatrs_domain::{JobScheduleState, JobScheduleStateOperations, NewJobScheduleState};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgJobScheduleStateRepository;

impl<'e> JobScheduleStateOperations<Transaction<'e, Postgres>> for PgJobScheduleStateRepository {
    type Err = DbError;

    /// Insert multiple job schedules
    async fn upsert_jobs_schedule(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        job_schedules: &[NewJobScheduleState],
    ) -> DbResult<()> {
        let db_job_schedules: Vec<DbJobScheduleState> =
            job_schedules.iter().map(|s| s.into()).collect();

        upsert_jobs_schedule(&mut **tx, &db_job_schedules).await?;

        Ok(())
    }
}

pub struct DbJobScheduleState {
    job_id: Uuid,
    next_run_at: DateTime<Utc>,
}

impl DbJobScheduleState {
    pub fn job_id_inner(&self) -> Uuid {
        self.job_id
    }

    pub fn next_run_at(&self) -> DateTime<Utc> {
        self.next_run_at
    }
}

impl From<&NewJobScheduleState> for DbJobScheduleState {
    fn from(value: &NewJobScheduleState) -> Self {
        DbJobScheduleState {
            job_id: value.job_id_inner(),
            next_run_at: value.next_run_at(),
        }
    }
}

impl From<NewJobScheduleState> for DbJobScheduleState {
    fn from(value: NewJobScheduleState) -> Self {
        DbJobScheduleState::from(&value)
    }
}

impl From<&JobScheduleState> for DbJobScheduleState {
    fn from(value: &JobScheduleState) -> Self {
        DbJobScheduleState {
            job_id: value.job_id_inner(),
            next_run_at: value.next_run_at().to_owned(),
        }
    }
}

impl From<JobScheduleState> for DbJobScheduleState {
    fn from(value: JobScheduleState) -> Self {
        DbJobScheduleState::from(&value)
    }
}
