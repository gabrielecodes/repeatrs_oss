use crate::error::ToServiceError;
use crate::{ServiceResult, err_ctx};
use chrono::{DateTime, Utc};
use repeatrs_bundles::JobQueueBundle;
use repeatrs_domain::{
    Job, JobOperations, JobQueueOperations, JobRunInsert, JobScheduleState, NewJobRun,
};
use repeatrs_transaction::{DatabaseContextProvider, TransactionError};
use std::marker::PhantomData;
use tracing::{info, instrument};

pub struct SchedulingService<E, B, D>
where
    B: JobQueueBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    bundle: B,
    database: D,
    _marker: PhantomData<E>,
}

impl<E, B, D> SchedulingService<E, B, D>
where
    B: JobQueueBundle<E> + Send + Sync + 'static,
    D: for<'tx> DatabaseContextProvider<'tx, E> + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    pub fn new(bundle: B, database: D) -> Self {
        Self {
            bundle,
            database,
            _marker: Default::default(),
        }
    }

    #[instrument(skip_all)]
    pub async fn dispatch_due_jobs(&self) -> ServiceResult<()> {
        let job_queue_repo = self.bundle.job_queue_repo();
        let job_repo = self.bundle.job_repo();

        let runnable_jobs = self
            .database
            .execute(|tx| {
                let job_repo = job_repo.clone();

                Box::pin(async move {
                    // BEGIN
                    // 1. SELECT due job_schedule_state FOR UPDATE SKIP LOCKED
                    // 2. compute next_run_at
                    // 3. INSERT job_runs (idempotent)
                    // 4. UPSERT job_schedule_state
                    // COMMIT

                    let runnable_jobs = err_ctx!(job_repo.get_due_jobs(tx).await)?;

                    if !runnable_jobs.is_empty() {
                        let job_runs_input: Vec<JobRunInsert> = runnable_jobs
                            .iter()
                            .map(|job: Job| {
                                let next_run_at = job.calculate_next_occurrence(j, false);
                                NewJobRun::new(job.job_id(), queue_id, scheduled_time)
                            })
                            .collect();

                        // insert new job_run
                        // upsert new job schedule state
                    }

                    Ok::<Vec<Job>, TransactionError>(runnable_jobs)
                })
            })
            .await
            .map_transaction_error(line!(), file!())?;

        if runnable_jobs.is_empty() {
            info!("No jobs are due to be executed.");
            return Ok(());
        }

        // let num_jobs = err_ctx!(job_queue_repo.publish(&runnable_jobs).await)?;

        // info!("{} jobs successfully dispatched.", num_jobs.len());

        Ok(())
    }
}
