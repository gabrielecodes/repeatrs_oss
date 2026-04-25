//! This module implements the central scheduling logic for the `repeatrs` service.
//! It orchestrates the execution of scheduled jobs, manages their lifecycle, and
//! handles interactions with the job storage and external commands. The scheduling
//! logic aims at scheduling jobs predictably with FIFO behavior.
//!
//! Key components and functionalities:
//! - Responsible for identifying jobs that are due for execution, and loading them
//!   in the `job_queues` table.
//! - Jobs are scheduled with FIFO logic, jobs ordered by priority first,  enqueue
//!   time second.
//! - Wakes up workers when the jobs are due to be executed
//! - If there are idle workers, the scheduler can move jobs from one queue to another
//!   to optimize resource usage

/*
TODO:
- add DAG directory check task,
- add reconciliation task
 */

use crate::ApiResult;
use crate::services::scheduling::SchedulingService;

use repeatrs_bundles::JobSchedulerBundle;
use repeatrs_transaction::DatabaseContextProvider;
use std::sync::Arc;
use tokio::sync::{Notify, watch};
use tracing::{error, info};

pub struct Scheduler<E, S, D>
where
    S: JobSchedulerBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    /// Handle to the job scheduling service.
    scheduling_service: SchedulingService<E, S, D>,

    /// Notification handle to wake up the scheduler.
    wakeup: Arc<Notify>,

    /// Receiver of the shutdown signal.
    shutdown: watch::Receiver<()>,
}

impl<E, S, D> Scheduler<E, S, D>
where
    S: JobSchedulerBundle<E> + Send + Sync + 'static,
    D: for<'tx> DatabaseContextProvider<'tx, E> + Send + Sync + 'static,
    E: Sync + Send + 'static,
{
    pub fn new(
        scheduling_service: SchedulingService<E, S, D>,
        wakeup: Arc<Notify>,
        shutdown: watch::Receiver<()>,
    ) -> Self {
        Self {
            scheduling_service,
            wakeup,
            shutdown,
        }
    }

    /// Scheduler loop. Schedules jobs when time is due and
    /// when jobs exit or are added/removed/deactivated.
    pub async fn start(&self) -> ApiResult<()> {
        let wakeup = self.wakeup.clone();
        let mut shutdown = self.shutdown.clone();

        loop {
            // schedule jobs
            if let Err(e) = self.scheduling_service.dispatch_due_jobs().await {
                error!("Error during job dispatch. {}", e);
            }

            // let deadline = Job::get_earliest_deadline(&pool).await;
            let sleep = tokio::time::sleep(tokio::time::Duration::from_mins(1));

            tokio::select! {
                    _ = sleep => {
                        info!("Running scheduling round.");
                    }

                    _ = wakeup.notified() => {
                        info!("Scheduler notified. Running scheduling round.")
                    }

                    _ = shutdown.changed() => {
                        info!("Scheduler received shutdown signal. Shutting down.");
                        break Ok(())
                    },
            }
        }
    }
}
