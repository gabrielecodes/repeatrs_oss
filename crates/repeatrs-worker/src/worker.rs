//! Workers manage the job lifecycle and VMs.
//!
//! ## Worker Logic
//!
//! Workers manage VMs and maintain a connection to them to check on their health status.
//! They update the `workers` table recording their own health and write to the `vm_heartbeats`
//! table, to record the health status of its VMs.
//!
//! ## Execution Logic
//!
//| Workers consume jobs from their queues and execute them until the maximum pool capacity
//| is reached. If a job due to be executed is already running and has
//| `max_concurrent_instances` = 1 it is skipped and not executed.
//|
//| Workers insert a `JobRun` to record each execution status. When a job terminates,
//| workers try to eagerly execute jobs waiting in the queue if there are any.
//!
//! Key components and functionalities:
//! - **Executor**: When notified, if sufficient resources are available on the VM, it fetches jobs
//!     from its queue
//! - **Status Reporting & Monitoring**: checks the status of the VM reporting its heartbeat to the
//!     database.
//! - **Status Handling**: It receives the heartbeat from the VM keeping a record it in the database.
//!     It starts a temination procedure for all jobs that do not report a heartbeat
//!     within a predefined interval.
//!

use crate::error::Result;

use bollard::Docker;
use bollard::query_parameters::{CreateContainerOptions, CreateImageOptionsBuilder};
use bollard::secret::ContainerCreateBody;
use chrono::{DateTime, Utc};
use futures_util::stream::StreamExt;
use sqlx::PgPool;
use std::default::Default;
use std::sync::Arc;
use tokio::sync::{Notify, watch};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct WorkerId(pub Uuid);

impl From<Uuid> for WorkerId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

#[derive(Default)]
enum ImagePullPolicy {
    /// Pull image at every job start
    #[default]
    Always,

    /// Pull image only if not present locally
    Missing,

    /// Never pull the image. Only use local images if present
    Never,
}

/// Responsible for the execution of jobs. Consumes jobs from its queue if it has sufficient
/// resources (memory, cpu). The worker has several responsibilities and functions:
///
/// - to consume jobs from the message queue and manage the lifecycle of the job container.
/// - Report heartbeats at regular intervals to the `workers.heartbeat.<worker_id>` topic.
/// - Monitor peak job resource consumption report them to the `job_runs` table at the end of
/// the job.
///
struct Worker {
    worker_id: WorkerId,
    epoch: i64,
    vpc_id: String,
    dns: String,
    private_ip: String,
    created_at: DateTime<Utc>,
}

impl Worker {
    /// Removes jobs from the queue
    pub async fn consume(&self) {
        // consme from the NATS queue
    }

    pub async fn run_container_task(
        &self,
        wakeup: Arc<Notify>,
        job: Job,
        run_id: Uuid,
        container_id: String,
        mut stop_rx: watch::Receiver<bool>,
        pool: PgPool,
        docker: Docker,
    ) -> Result<()> {
        // let mut guard = SlotGuard::new(pool.clone(), job.pool_id(), wakeup);
        // let mut heartbeat = time::interval(Duration::from_secs(30));

        // let wait_stream = self.pull_or_get_image(&docker, image);
        // TODO: start container with fixed name e.g. container_id + run_id
        // let container = self.start_or_attach_container();

        // loop {
        //     tokio::select! {
        //         _ = heartbeat.tick() => {
        //             let _ = sqlx::query!(
        //                 "INSERT INTO job_liveness (run_id, last_ping)
        //              VALUES ($1, NOW())
        //              ON CONFLICT (run_id) DO UPDATE SET last_ping = EXCLUDED.last_ping",
        //                 run_id
        //             ).execute(&pool).await;
        //         }

        //         // container exits with success or error
        //         exit_res = wait_stream => {
        //             match exit_res {
        //                 Ok(exit) => {
        //                     // update_db set to failed or completed only after verifying
        //                     println!("Container {} exited with code {}", container_id, exit.status_code);
        //                     // Finalize the job_runs entry here
        //                     return Ok(());
        //                 }
        //                 Err(e) => return Err(e.into()),
        //             }
        //         }

        //         // cancellation signal
        //         _ = stop_rx.changed() => {
        //             println!("Cancellation received for run_id {}. Killing container...", run_id);
        //             let _ = docker.containers().get(&container_id).kill(None).await;
        //             // update db: set to stopped only after verifying
        //             return Ok(());
        //         }
        //     }
        // }

        Ok(())
    }

    async fn pull_or_get_image(&self, docker: &Docker, image: &str) -> Result<()> {
        // 1. Check if image exists locally to skip the network call
        // This saves bandwidth and reduces "pull rate limiting" on Docker Hub
        if docker.inspect_image(image).await.is_ok() {
            return Ok(());
        }

        tracing::info!("Image {} not found locally. Pulling...", image);

        let options = CreateImageOptionsBuilder::default()
            .from_image(image)
            .build();

        // 2. Start the pull stream
        let mut pull_stream = docker.create_image(Some(options), None, None);

        while let Some(pull_result) = pull_stream.next().await {
            match pull_result {
                Ok(progress) => {
                    // You can log progress.status or progress.progress here
                    // if you want to expose pull-status to your users
                }
                Err(e) => return Err(crate::error::Error::Unknown(format!("Error: {}", e))),
            }
        }

        Ok(())
    }

    // if a job is RUNNING but there is no task running it (e.g. it crashed)
    // the task is spawned trying to create the container. It gets 409 conflict
    // get the container id and resume the heartbeat loop
    pub async fn start_or_attach_container(
        &self,
        docker: &bollard::Docker,
        image: &str,
        run_id: i64,
        command: Vec<String>,
    ) -> Result<String> {
        let container_name = format!("job_run_{}", run_id);

        let _config = ContainerCreateBody {
            image: Some(image.to_string()),
            cmd: Some(command),
            host_config: Some(bollard::models::HostConfig {
                auto_remove: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let _create_options = CreateContainerOptions {
            name: Some(container_name.clone()),
            ..Default::default()
        };

        Ok("".to_string())

        // // 1. Attempt to create
        // match docker.create_container(Some(create_options), config).await {
        //     Ok(container) => {
        //         // Success: Now start it
        //         docker.start_container(&container.id, None).await.unwrap();
        //         Ok(container.id)
        //     }
        //     Err(Error::DockerResponseConflictError { .. }) => {
        //         // 2. Conflict: The container name already exists. Re-attach.
        //         println!(
        //             "Container {} already exists. Re-attaching...",
        //             container_name
        //         );

        //         let inspection = docker
        //             .inspect_container(&container_name, None::<InspectContainerOptions>)
        //             .await?;

        //         let id = inspection.id.ok_or(Error::IOError {
        //             err: std::io::Error::new(std::io::ErrorKind::NotFound, "ID missing"),
        //         })?;

        //         // Check if it's already running; if not, start it.
        //         if let Some(state) = inspection.state {
        //             if state.running != Some(true) {
        //                 docker.start_container(&id, None).await?;
        //             }
        //         }

        //         Ok(id)
        //     }
        //     Err(e) => Err(e),
        // }
    }
}
