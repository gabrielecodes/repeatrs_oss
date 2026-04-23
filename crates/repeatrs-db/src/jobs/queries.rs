use crate::{
    DbResult,
    jobs::{DbJobId, DbJobStatus, JobRow},
    queues::DbQueueId,
};

use repeatrs_domain::NewJob;
use sqlx::{Executor, Postgres};

pub(crate) async fn add_job<'e, E>(exec: E, job: &NewJob, queue_id: &DbQueueId) -> DbResult<DbJobId>
where
    E: Executor<'e, Database = Postgres>,
{
    // let schedule = Cron::from_str(&job.schedule)?;
    // let next_occurrence = schedule.find_next_occurrence(&Utc::now(), false)?;

    let job_id: DbJobId = sqlx::query_scalar!(
        r#"INSERT INTO jobs (
                job_name,
                description,
                schedule,
                options,
                image_name,
                command,
                args,
                max_retries,
                priority,
                queue_id,
                max_concurrency,
                timeout_seconds
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING
                job_id AS "job_id: DbJobId"
            "#,
        job.job_name,
        job.description,
        job.schedule.to_string(),
        job.options,
        job.image_name,
        job.command,
        job.args,
        job.max_retries,
        job.priority,
        queue_id as &DbQueueId,
        job.max_concurrency,
        job.timeout_seconds,
    )
    .fetch_one(exec)
    .await?;

    Ok(job_id)
}

/// Returns true if the job with the given id already exists.
pub(crate) async fn get_job_by_id<'e, E>(exec: E, job_id: &DbJobId) -> DbResult<JobRow>
where
    E: Executor<'e, Database = Postgres>,
{
    let job: JobRow = sqlx::query_as!(
        JobRow,
        r#"
            SELECT
                j.job_id as "job_id: DbJobId",
                j.job_name,
                j.description,
                j.schedule,
                j.options,
                j.image_name,
                j.command,
                j.args,
                j.max_retries,
                j.status AS "status: DbJobStatus",
                j.priority,
                j.queue_id as "queue_id: DbQueueId",
                j.max_concurrency,
                j.timeout_seconds,
                j.created_at,
                j.updated_at,
                q.queue_name
            FROM jobs j
            INNER JOIN queues q ON j.queue_id = q.queue_id
            WHERE job_id = $1"#,
        &job_id as &DbJobId,
    )
    .fetch_one(exec)
    .await?;

    Ok(job)
}

/// Returns true if the job with the given name already exists.
pub(crate) async fn get_job_by_name<'e, E>(exec: E, job_name: &str) -> DbResult<JobRow>
where
    E: Executor<'e, Database = Postgres>,
{
    let job = sqlx::query_as!(
        JobRow,
        r#"
            SELECT
                j.job_id as "job_id: DbJobId",
                j.job_name,
                j.description,
                j.schedule,
                j.options,
                j.image_name,
                j.command,
                j.args,
                j.max_retries,
                j.status AS "status: DbJobStatus",
                j.priority,
                j.queue_id as "queue_id: DbQueueId",
                j.max_concurrency,
                j.timeout_seconds,
                j.created_at,
                j.updated_at,
                q.queue_name
            FROM jobs j
            INNER JOIN queues q ON j.queue_id = q.queue_id
            WHERE job_name = $1"#,
        job_name
    )
    .fetch_one(exec)
    .await?;

    Ok(job)
}

pub(crate) async fn get_jobs_by_queue_id<'e, E>(
    exec: E,
    queue_id: &DbQueueId,
) -> DbResult<Vec<JobRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let jobs: Vec<JobRow> = sqlx::query_as!(
        JobRow,
        r#"
            SELECT
                j.job_id as "job_id: DbJobId",
                j.job_name,
                j.description,
                j.schedule,
                j.options,
                j.image_name,
                j.command,
                j.args,
                j.max_retries,
                j.status AS "status: DbJobStatus",
                j.priority,
                j.queue_id as "queue_id: DbQueueId",
                j.max_concurrency,
                j.timeout_seconds,
                j.created_at,
                j.updated_at,
                q.queue_name
            FROM jobs j            
            INNER JOIN queues q ON j.queue_id = q.queue_id
            WHERE j.queue_id = $1
            "#,
        queue_id as &DbQueueId
    )
    .fetch_all(exec)
    .await?;

    Ok(jobs)
}

pub(crate) async fn get_jobs_by_queue_name<'e, E>(
    exec: E,
    queue_name: &str,
) -> DbResult<Vec<JobRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let jobs: Vec<JobRow> = sqlx::query_as!(
        JobRow,
        r#"
        SELECT
            j.job_id as "job_id: DbJobId",
            j.job_name,
            j.description,
            j.schedule,
            j.options,
            j.image_name,
            j.command,
            j.args,
            j.max_retries,
            j.status AS "status: DbJobStatus",
            j.priority,
            j.queue_id as "queue_id: DbQueueId",
            j.max_concurrency,
            j.timeout_seconds,
            j.created_at,
            j.updated_at,
            q.queue_name
        FROM jobs j
        INNER JOIN queues q ON j.queue_id = q.queue_id
        WHERE queue_name = $1
        "#,
        queue_name
    )
    .fetch_all(exec)
    .await?;

    Ok(jobs)
}

// pub(crate) async fn get_earliest_deadline<'e, E>(exec: E) -> Instant
// where
//     E: Executor<'e, Database = Postgres>,
// {
//     let record: std::result::Result<
//         Option<sqlx::types::chrono::DateTime<Utc>>,
//         sqlx::error::Error,
//     > = sqlx::query_scalar!(
//         r#"
//         SELECT
//             COALESCE(
//                 MIN(next_occurrence_at),
//                 NOW() + INTERVAL '1 minute')::timestamp AT TIME ZONE 'UTC'
//         FROM job_schedule_state
//         "#
//     )
//     .fetch_one(exec)
//     .await;

//     match record {
//         Ok(Some(d)) => d
//             .signed_duration_since(Utc::now())
//             .to_std()
//             .map(|d| Instant::now() + d)
//             .unwrap_or(Instant::now() + Duration::from_mins(1)),
//         _ => Instant::now() + Duration::from_mins(1),
//     }
// }

/// Returns active jobs whose time is due.
pub(crate) async fn get_due_jobs<'e, E>(exec: E) -> DbResult<Vec<JobRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let jobs: Vec<JobRow> = sqlx::query_as!(
        JobRow,
        r#"
        SELECT
            j.job_id as "job_id: DbJobId",
            j.job_name,
            j.description,
            j.schedule,
            j.options,
            j.image_name,
            j.command,
            j.args,
            j.max_retries,
            j.status AS "status: DbJobStatus",
            j.priority,
            j.queue_id,
            j.max_concurrency,
            j.timeout_seconds,
            j.created_at,
            j.updated_at,
            q.queue_name
        FROM jobs j
        INNER JOIN queues q ON j.queue_id = q.queue_id
        INNER JOIN job_schedule_state js ON j.job_id = js.job_id
        WHERE js.next_run_at <= NOW()
            AND j.status = 'ACTIVE'
        ORDER BY js.next_run_at
        FOR UPDATE OF j SKIP LOCKED
        "#,
    )
    .fetch_all(exec)
    .await?;

    //TODO: add global limit to the query above

    Ok(jobs)
}

// /// Updates the next occurrence timestamp of the jobs corresponding to the given [`Job`]s.
// pub(crate) async fn update_deadlines<'e, E>(exec: E, jobs: &[JobRow]) -> DbResult<()>
// where
//     E: Executor<'e, Database = Postgres>,
// {
//     let capacity = jobs.len();
//     let mut deadlines = Vec::with_capacity(capacity);
//     let mut ids = Vec::with_capacity(capacity);

//     for job in jobs {
//         deadlines.push(
//             job.calculate_next_occurrence(false)
//                 .map_err(DbError::Cron)?,
//         );
//         ids.push(job.job_id().0);
//     }

//     sqlx::query!(
//         r#"
//         UPDATE jobs j
//         SET next_occurrence_at = data.next_time
//         FROM (
//             SELECT
//                 UNNEST($1::uuid[])        AS job_id,
//                 UNNEST($2::timestamptz[]) AS next_time
//         ) AS data
//         WHERE j.job_id = data.job_id;
//         "#,
//         &ids,
//         &deadlines
//     )
//     .execute(exec)
//     .await?;

//     Ok(())
// }

impl JobRow {
    /// Sets the job to [`JobStatus::Inactive`].
    pub async fn deactivate_job_by_id<'e, E>(&self, exec: E) -> DbResult<DbJobId>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id: DbJobId = sqlx::query_scalar!(
            r#"
            UPDATE jobs 
                SET status = 'INACTIVE'::job_status 
            WHERE job_id = $1 
              AND status = 'ACTIVE'::job_status
            RETURNING job_id AS "job_id: DbJobId"
            "#,
            self.job_id() as DbJobId
        )
        .fetch_one(exec)
        .await?;

        // TODO: remove job from job queue

        Ok(id)
    }

    /// Sets the job to [`JobStatus::Inactive`].
    pub async fn deactivate_job_by_name<'e, E>(&self, exec: E) -> DbResult<DbJobId>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id: DbJobId = sqlx::query_scalar!(
            r#"
            UPDATE jobs 
                SET status = 'INACTIVE'::job_status 
            WHERE job_name = $1 
                AND status = 'ACTIVE'::job_status
            RETURNING job_id AS "job_id: DbJobId"
                "#,
            self.job_name()
        )
        .fetch_one(exec)
        .await?;

        // TODO: remove job from job queue

        Ok(id)
    }

    pub async fn delete_job_by_id<'e, E>(&self, exec: E) -> DbResult<DbJobId>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id: DbJobId = sqlx::query_scalar!(
            r#"DELETE FROM jobs WHERE job_id = $1 RETURNING job_id AS "job_id: DbJobId""#,
            self.job_id() as DbJobId
        )
        .fetch_one(exec)
        .await?;

        Ok(id)
    }

    pub async fn delete_job_by_name<'e, E>(&self, exec: E) -> DbResult<DbJobId>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id: DbJobId = sqlx::query_scalar!(
            r#"DELETE FROM jobs WHERE job_name = $1 RETURNING job_id AS "job_id: DbJobId""#,
            self.job_name()
        )
        .fetch_one(exec)
        .await?;

        Ok(id)
    }
}

// #[tracing::instrument(skip_all, fields(job=%self.name()))]
// pub async fn execute(&self, mut shutdown_rx: watch::Receiver<()>) -> DbResult<()> {
//     // let shell = "/bin/sh";
//     // let arg = "-c";

//     let mut command = Command::new("docker run");
//     command
//         .arg(self.options)
//         .arg(self.command())
//         .stdout(std::process::Stdio::piped())
//         .stderr(std::process::Stdio::piped());

//     let mut child = command.spawn()?;

//     let Some(stdout) = child.stdout.take() else {
//         return Err(crate::db::Error::Custom(
//             "Could not capture stdout".to_string(),
//         ));
//     };

//     let Some(stderr) = child.stderr.take() else {
//         return Err(crate::db::Error::Custom(
//             "Could not capture stderr".to_string(),
//         ));
//     };

//     let mut logger = Logger::new()
//         .with_source(stdout)
//         .with_source(stderr)
//         .with_destination(LogDestination::file())
//         .start()
//         .await;

//     let timeout_duration = Duration::from_secs(self.timeout_seconds() as u64);
//     let mut heartbeat = get_heartbeat();

//     tokio::select! {
//         _ = shutdown_rx.changed() => {
//             info!("Shutdown signal received.");
//             // update job_run status to Stopping
//             let _ = child.kill().await;
//             // update job_run status to Stopped
//             return Ok(())
//         }

//         _ = heartbeat.tick() => {
//             // update job_run table
//             info!("heartbeat");
//         }

//         result = timeout(timeout_duration, child.wait()) => {
//             let status = match result {
//                 Ok(Ok(s)) => Ok(s),
//                 Ok(Err(e)) => Err(e),
//                 Err(_) => {
//                     if let Err(e) = child.kill().await {
//                         error!("Failed to kill timed-out job: {}.", e);
//                     }
//                     let msg = "Job timed out and was killed.";
//                     error!(error = msg);
//                     return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, msg).into());
//                 }
//             };

//             let msg = match status {
//                 Ok(s) if s.success() => format!("Job completed successfully with {}.", s),
//                 Ok(s) => format!("Job failed with {}.", s),
//                 Err(ref e) => format!("Job failed to execute: {}.", e),
//             };
//             info!(msg);

//             // if logger.send_message(&msg).await.is_err() {
//             //     error!(error = "Could not write exit status to job logs.");
//             // };

//             // let _ = tokio::join!(stdout_task, stderr_task);
//         }
//     }

//     Ok(())
// }
