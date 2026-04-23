use crate::error::ToApiError;
use crate::{ApiResult, err_ctx};
use repeatrs_bundles::JobSchedulerBundle;
use repeatrs_domain::{
    JobOperations, JobRunOperations, JobScheduleStateOperations, NewJobRun, NewJobScheduleState,
};
use repeatrs_transaction::{DatabaseContextProvider, TransactionError};
use std::marker::PhantomData;
use tracing::{info, instrument};

pub struct SchedulingService<E, B, D>
where
    B: JobSchedulerBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    bundle: B,
    database: D,
    _marker: PhantomData<E>,
}

impl<E, B, D> SchedulingService<E, B, D>
where
    B: JobSchedulerBundle<E> + Send + Sync + 'static,
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
    pub async fn dispatch_due_jobs(&self) -> ApiResult<()> {
        let job_repo = self.bundle.job_repo();
        let job_runs_repo = self.bundle.job_runs_repo();
        let job_schedule_state_repo = self.bundle.job_schedule_state_repo();

        let runnable_jobs = self
            .database
            .execute(|tx| {
                let job_repo = job_repo.clone();
                let job_runs_repo = job_runs_repo.clone();
                let job_schedule_state_repo = job_schedule_state_repo.clone();

                Box::pin(async move {
                    let jobs = err_ctx!(job_repo.get_due_jobs(tx).await)?;

                    if jobs.is_empty() {
                        return Ok(Vec::new());
                    }

                    let job_runs_input: Vec<NewJobRun> = jobs
                    .iter()
                    .filter_map(|job| {
                        match job.calculate_next_occurrence(false) {
                            Ok(next) => Some(NewJobRun::new(job.job_id(), job.queue_id(), &next)),
                            Err(e) => {
                                tracing::error!(job_id = %job.job_id(), error = %e, "Failed to calculate next run");
                                None
                            }
                        }
                    })
                    .collect();

                    if !job_runs_input.is_empty() {
                        err_ctx!(job_runs_repo.insert_job_runs(tx, &job_runs_input).await)?;

                        let v: Vec<NewJobScheduleState> = job_runs_input.iter().map(|job_run| NewJobScheduleState::new(job_run.job_id(), job_run.scheduled_time())).collect();

                        err_ctx!(job_schedule_state_repo.upsert_jobs_schedule(tx, &v).await)?;
                    }

                    Ok::<Vec<NewJobRun>, TransactionError>(job_runs_input)
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
