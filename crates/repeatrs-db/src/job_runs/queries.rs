use repeatrs_domain::NewJobRun;
use sqlx::{Executor, Postgres};

use crate::DbResult;
use crate::job_runs::{DbJobRunId, DbJobRunStatus, JobRunRow};
use crate::jobs::DbJobId;
use crate::queues::DbQueueId;
use crate::workers::DbWorkerId;
use chrono::{DateTime, Utc};

pub(crate) async fn insert_job_runs<'e, E>(
    exec: E,
    job_info: &[NewJobRun],
) -> DbResult<Vec<DbJobRunId>>
where
    E: Executor<'e, Database = Postgres>,
{
    if job_info.is_empty() {
        return Ok(Vec::new());
    }

    let job_ids: Vec<DbJobId> = job_info.iter().map(|j| j.job_id().into()).collect();
    let next_run_times: Vec<DateTime<Utc>> = job_info.iter().map(|j| j.scheduled_time()).collect();

    let run_ids: Vec<DbJobRunId> = sqlx::query!(
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
        RETURNING job_run_id as "job_run_id: DbJobRunId"
        "#,
        &job_ids as &[DbJobId],
        &next_run_times as &[DateTime<Utc>],
    )
    .fetch_all(exec)
    .await?
    .into_iter()
    .map(|r| r.job_run_id)
    .collect();

    Ok(run_ids)
}

/// Returns the jobs in [`RunStatus::RUNNING`].
pub(crate) async fn get_running_jobs<'e, E>(exec: E) -> DbResult<Vec<JobRunRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let currently_running = sqlx::query_as!(
        JobRunRow,
        r#"
        SELECT
            job_run_id as "job_run_id: DbJobRunId",
            job_id as "job_id: DbJobId",
            queue_id as "queue_id: DbQueueId",
            worker_id as "worker_id: DbWorkerId",
            claimed_at,
            status as "status: DbJobRunStatus",
            scheduled_time,
            attempt_count,
            started_at,
            ended_at,
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

pub(crate) async fn get_job_runs_by_job_id<'e, E>(
    exec: E,
    job_id: DbJobId,
) -> DbResult<Vec<JobRunRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let job_run: Vec<JobRunRow> = sqlx::query_as!(
        JobRunRow,
        r#"
        SELECT
            job_run_id as "job_run_id: DbJobRunId",
            job_id as "job_id: DbJobId",
            queue_id as "queue_id: DbQueueId",
            worker_id as "worker_id: DbWorkerId",
            claimed_at,
            status as "status: DbJobRunStatus",
            scheduled_time,
            attempt_count,
            started_at,
            ended_at,
            exit_code,
            error,
            created_at,
            updated_at
        FROM job_runs
        WHERE job_id = $1
        LIMIT 100
        "#,
        job_id as DbJobId
    )
    .fetch_all(exec)
    .await?;

    Ok(job_run)
}

pub(crate) async fn get_job_runs_by_queue_id<'e, E>(
    exec: E,
    queue_id: DbQueueId,
) -> DbResult<Vec<JobRunRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let job_run: Vec<JobRunRow> = sqlx::query_as!(
        JobRunRow,
        r#"
        SELECT
            job_run_id as "job_run_id: DbJobRunId",
            job_id as "job_id: DbJobId",
            queue_id as "queue_id: DbQueueId",
            worker_id as "worker_id: DbWorkerId",
            claimed_at,
            status as "status: DbJobRunStatus",
            scheduled_time,
            attempt_count,
            started_at,
            ended_at,
            exit_code,
            error,
            created_at,
            updated_at
        FROM job_runs
        WHERE job_id = $1
        LIMIT 100
        "#,
        queue_id as DbQueueId
    )
    .fetch_all(exec)
    .await?;

    Ok(job_run)
}

pub(crate) async fn get_latest_job_runs<'e, E>(exec: E) -> DbResult<Vec<JobRunRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let job_run: Vec<JobRunRow> = sqlx::query_as!(
        JobRunRow,
        r#"
        SELECT
            job_run_id as "job_run_id: DbJobRunId",
            job_id as "job_id: DbJobId",
            queue_id as "queue_id: DbQueueId",
            worker_id as "worker_id: DbWorkerId",
            claimed_at,
            status as "status: DbJobRunStatus",
            scheduled_time,
            attempt_count,
            started_at,
            ended_at,
            exit_code,
            error,
            created_at,
            updated_at
        FROM job_runs
        ORDER BY updated_at
        LIMIT 100
        "#,
    )
    .fetch_all(exec)
    .await?;

    Ok(job_run)
}
