use repeatrs_domain::{JobId, JobRunId, JobScheduleState};
use sqlx::{Executor, Postgres};

use crate::DbResult;
use crate::workers::DbWorkerId;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(crate) async fn insert_many<'e, E>(
    exec: E,
    job_ids: &[Uuid],
    next_run_times: &[DateTime<Utc>],
) -> DbResult<Vec<JobRunId>>
where
    E: Executor<'e, Database = Postgres>,
{
    if job_ids.is_empty() {
        return Ok(Vec::new());
    }

    let run_ids = sqlx::query!(
        r#"
        INSERT INTO job_runs (
            job_run_id,
            job_id,
            scheduled_time
        )
        SELECT
            gen_random_uuid(),
            data.job_id,
            data.scheduled_time
        FROM UNNEST($1::uuid[], $2::timestamptz[]) AS data(job_id, scheduled_time)
        ON CONFLICT (job_id, scheduled_time) DO NOTHING
        RETURNING job_run_id as "job_run_id: JobRunId"
        "#,
        job_ids,
        next_run_times,
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(|r| r.job_run_id)
    .collect();

    Ok(run_ids)
}

/// Returns the jobs in [`RunStatus::RUNNING`].
pub(crate) async fn get_running_jobs<'e, E>(exec: E) -> DbResult<Vec<JobRunId>>
where
    E: Executor<'e, Database = Postgres>,
{
    let currently_running = sqlx::query_as!(
        JobRun,
        r#"
        SELECT
            job_run_id,
            job_id,
            queue_id,
            worker_id as "worker_id: DbWorkerId",
            claimed_at,
            status as "status: DbJobRunStatus",
            next_occurrence_at,
            attempt_count,
            started_at,
            ended_at, 
            duration_secs,
            exit_code,
            error, 
            created_at,
            updated_at
        FROM job_runs
        WHERE status = 'RUNNING'
    "#
    )
    .fetch_all(exec)
    .await?;

    Ok(currently_running)
}

pub(crate) async fn get_job_run_by_id<'e, E>(exec: E, run_id: JobRunId) -> Result<JobRun>
where
    E: Executor<'e, Database = Postgres>,
{
    let job_run = sqlx::query_as!(
        JobRun,
        r#"
        SELECT
            job_run_id,
            job_id,
            worker_id as "worker_id: DbWorkerId",
            container_name,
            status as "status: JobRunStatus",
            started_at,
            ended_at,
            duration_secs,
            exit_code,
            error,
            created_at,
            updated_at
        FROM job_runs
        WHERE job_run_id = $1
        "#,
        run_id as JobRunId
    )
    .fetch_one(exec)
    .await?;

    Ok(job_run)
}
