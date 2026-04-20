//! Worekr failover logic.
//!
//! The failover logic takes into account several scenarios.
//! - **Job exits with error**:
//!     - Read the exit code.
//!     - If retries < max_retries, increment the retry count, calculate the backoff, and re-queue the job or wait
//!          to restart it.
//!     - If retries >= max_retries, mark the job_run as failed in the db.
//!
//! - **Worker terminates prematurely**:
//!     - No heartbeat for e.g. 120 seconds. The job container is now orphaned and stuck in RUNNING state.
//!     - The worker process is restarted by its supervisor (e.g., systemd).
//!     - The worker queries PostgreSQL for any job_run that is marked as running and assigned to its own worker_id.
//!     - For each job, it must determine the container's true state using the container runtime:
//!         * Case A: Container is still running. The worker "re-attaches" it and start monitoring it again and
//!           proceed as normal.
//!         * Case B: Container has exited. The worker inspects the exited container to get the final exit
//!           code and finish time. It can then process the result (success, retry, or fail) and update the db
//!           correctly.
//!         * Case C: Container does not exist. This implies it was already cleaned up or never started properly.
//!           The job is marked as failed and re-queued.
//!
//! - **Database connectivity lost**:
//!     - The worker continues doing its job and holds on to the final result trying to reconnect and load
//!       the results indefinitely

use futures::future::join_all;
use sqlx::PgPool;
use tokio::time::Duration;

use crate::db::ExecutorHeartbeat;
use crate::worker::WorkerId;

/// Checks the status of the executors and updates the `executor_heartbeats` table.
/// The executors that are in status `UNREACHABLE`
async fn worker_loop(worker_id: WorkerId, epoch: u32, pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        let executors = sqlx::query_as!(
            Executor,
            "SELECT * FROM executors WHERE worker_id = $1",
            worker_id
        )
        .fetch_all(pool)
        .await
        .unwrap();

        // poll executors
        let updates: Vec<ExecutorHeartbeat> = join_all(executors.iter().map(|exec| async move {
            // Attempt to reach the VM (e.g., via a lightweight HTTP health check or SSH)
            match exec.check_vm_health().await {
                Ok(status) => ExecutorHeartbeat {
                    executor_id: exec.executor_id,
                    status,
                },
                Err(_) => ExecutorHeartbeat {
                    vm_id: vm.vm_id,
                    status: "UNREACHABLE".to_string(),
                },
            }
        }))
        .await;

        // batch update the status of the alive executors only
        // we "stale" unreachable executors by not updating their heartbeat
        let alive_updates: Vec<_> = updates.into_iter().filter(|u| u.alive).collect();

        if !alive_updates.is_empty() {
            if let Err(e) = flush_heartbeats_to_db(&pool, worker_id, epoch, alive_updates).await {
                eprintln!("Failed to flush heartbeats: {}", e);
            }
        }
    }
}

async fn flush_heartbeats_to_db(
    pool: &PgPool,
    worker_id: uuid::Uuid,
    epoch: i32,
    updates: Vec<VmStateUpdate>,
) -> Result<(), sqlx::Error> {
    let vm_ids: Vec<uuid::Uuid> = updates.iter().map(|u| u.vm_id).collect();
    let statuses: Vec<String> = updates.iter().map(|u| u.status.clone()).collect();

    // Batch Upsert into the UNLOGGED table
    sqlx::query!(
        r#"
        INSERT INTO vm_heartbeats (vm_id, worker_id, job_run_id, epoch, status, last_seen)
        SELECT 
            unnest($1::uuid[]), 
            $2, 
            $3, -- simplified for example
            $4, 
            unnest($5::text[]), 
            NOW()
        ON CONFLICT (vm_id) 
        DO UPDATE SET 
            status = EXCLUDED.status,
            last_seen = NOW(),
            epoch = EXCLUDED.epoch
        "#,
        &vm_ids,
        worker_id,
        job_run_id, // map from local state
        epoch,
        &statuses
    )
    .execute(pool)
    .await?;

    Ok(())
}
/*
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::time::{self, Duration, Interval};
use uuid::Uuid;

use crate::{config::get_config, db::Result};

struct JobLiveness {
    run_id: Uuid,
    last_ping: Option<DateTime<Utc>>,
}

pub struct FailoverHandler {
    pool: PgPool,
    interval: Interval,
    threshold: u64,
}

impl FailoverHandler {
    pub fn new(pool: PgPool) -> Self {
        let config = get_config();
        let heartbit = config.heartbeat_interval_seconds();
        let threshold = config.heartbeat_threshold_seconds();

        let interval = time::interval(Duration::from_secs(heartbit));

        Self {
            pool,
            interval,
            threshold,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.interval.tick().await;

        let mut conn = self.pool.acquire().await?;

        loop {
            self.interval.tick().await;

            let stale_jobs: Vec<JobLiveness> = sqlx::query_as!(
                JobLiveness,
                "SELECT run_id, last_ping FROM job_liveness
                WHERE last_ping < NOW() - ($1 * INTERVAL '1 second')",
                self.threshold as f64
            )
            .fetch_all(&mut *conn)
            .await?;

            let stale_job_ids: Vec<Uuid> = stale_jobs.iter().map(|j| j.run_id).collect();

            // terminate/kill stale_jobs
            // for id in stale_jobs...
            // once each container is confirmed dead -> update status

            if !stale_jobs.is_empty() {
                let failed_jobs = sqlx::query!(
                    r#"
                    UPDATE job_runs
                    SET 
                        status = 'FAILED', 
                        ended_at = NOW(), 
                        error = 'Heartbeat timeout'
                    WHERE run_id = ANY($1)
                    RETURNING job_id
                    "#,
                    &stale_job_ids
                )
                .fetch_all(&mut *conn)
                .await?;

                let failed_job_ids: Vec<Uuid> = failed_jobs.iter().map(|j| j.job_id).collect();

                sqlx::query!(
                    r#"
                    -- reconcile slots usage
                    UPDATE pools p
                    SET available_slots = p.capacity - (
                        SELECT COALESCE(SUM(j.required_slots), 0)
                        FROM job_runs jr
                        INNER JOIN jobs j USING(job_id)
                        WHERE jr.job_id = ANY($1) 
                            AND p.pool_id = j.pool_id
                    )"#,
                    &failed_job_ids
                )
                .fetch_all(&mut *conn)
                .await?;
            }

            // TODO: send kill SIGTERM/SIGKILL to failed jobs
        }
    }
}

/*
An Orphaned Job is defined as a job_run that is not in a termin
al state (COMPLETED, FAILED, STOPPED), but whose associated worker has failed to heartbeat recently.

SELECT
    jr.id AS run_id,
    jr.job_id,
    jr.worker_id,
    jr.status,
    w.last_heartbeat
FROM job_runs jr
JOIN workers w ON jr.worker_id = w.id
WHERE
    -- Only check active jobs
    jr.status IN ('PENDING', 'RUNNING')
    -- A worker is considered "down" if no heartbeat in 30 seconds
    AND w.last_heartbeat < NOW() - INTERVAL '30 seconds'
    -- (Optional) Logic to ignore jobs where Vector metrics are still flowing
    -- unless you are explicitly trying to reattach the worker.
    AND jr.id NOT IN (
        /* subquery or join with your Vector metrics metadata table */
    );
*/

*/
