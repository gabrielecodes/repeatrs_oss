use repeatrs_db::workers::{Worker, WorkerHeartbeat, WorkerId, WorkerStatus};
use sqlx::PgPool;
use tokio::time::Duration;

pub struct HealthController {
    pool: PgPool,
}

impl HealthController {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Starts the monitoring service that periodically checks workers' health:
    /// - if a worker is unresponsive, check the status of the VM
    /// - if the VM is unresponsive, terminate and reprovision
    /// - if tthe VM is responsive, restart the worker and reattach the containers
    pub async fn start_workers_monitor(state: State) {
        let duration = Duration::from_secs(10);
        let health_controller = HealthController::new(self.state.pool.clone());

        loop {
            // query the heartbeats every 10 seconds and check for unresponsive workers
            let worker_status = self.get_heartbeats().await?;

            // if a worker is unresponsive, check the status of the VM
            // if the VM is unresponsive, terminate and reprovision
            // if the VM is responsive, restart the worker and reattach the containers

            sleep(duration).await;
        }
    }

    pub async fn get_heartbeats(&self) -> Result<Vec<WorkerId>> {
        let tx = self.pool.begin().await?;

        // get all worker ids of alive workers
        let alive_workers = Worker::get_worker_by_status(&mut tx, WorkerStatus::Alive).await?;

        // check their heartbeat
        let heartbeats = WorkerHeartbeat::get_unresponsive_workers(
            &mut tx,
            Duration::from_secs(30),
            &alive_workers,
        )
        .await?;

        tx.commit().await?;

        Ok(heartbeats)
    }
}
