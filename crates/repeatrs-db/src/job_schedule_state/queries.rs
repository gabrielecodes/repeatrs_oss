use crate::{DbResult, job_schedule_state::DbJobScheduleState};

use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub(crate) async fn upsert_jobs_schedule<'e, E>(
    exec: E,
    jobs_schedules: &[DbJobScheduleState],
) -> DbResult<()>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    if jobs_schedules.is_empty() {
        return Ok(());
    }

    let job_ids: Vec<Uuid> = jobs_schedules.iter().map(|j| j.job_id).collect();
    let next_times: Vec<DateTime<Utc>> = jobs_schedules.iter().map(|j| j.next_run_at).collect();

    sqlx::query!(
        r#"
        INSERT INTO job_schedule_state (
            job_id,
            next_run_at,
            last_scheduled_at
        )
        SELECT
            data.job_id,
            data.next_time,
            NULL
        FROM UNNEST($1::uuid[], $2::timestamptz[]) AS data(job_id, next_time)
        ON CONFLICT (job_id) DO UPDATE
        SET
            last_scheduled_at = job_schedule_state.next_run_at,
            next_run_at = EXCLUDED.next_run_at;
        "#,
        &job_ids,
        &next_times,
    )
    .execute(exec)
    .await?;

    Ok(())
}
