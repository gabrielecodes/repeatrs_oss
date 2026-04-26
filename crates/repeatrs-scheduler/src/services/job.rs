//! Implementation of business logic/invariants for the [`Job`] entity.
//!
//! The service layer is responsible for:
//!
//! - Implementing business logic and invariants related to [`Job`] management. Rules
//!   that involves multiple entities are enforced. These include:
//!     - Uniqueness or existence checks,
//!     - Aggregate rules (e.g. respecting limits/quotas involving multiple entities).
//!     - Workflow rules (e.g. forbidden state changes or entity creation).
//! - Interacting with the repository to perform database operations.
//! - Providing a clear API for controllers.

use crate::{ApiResult, controllers::job::CreateJobCommand, err_ctx, error::ToApiError};
use repeatrs_bundles::JobBundle;
use repeatrs_domain::{
    ContainerRunCommand, Job, JobId, JobIdentity, JobOperations, JobOptions, JobResponse, QueueId,
    QueueOperations,
};
use repeatrs_proto::repeatrs::JobItem;
use repeatrs_transaction::{DatabaseContextProvider, TransactionError};
use std::marker::PhantomData;
use tracing::{Span, field::Empty, info, instrument};

/// Defines the service for job related operations.
pub struct JobService<E, B, D>
where
    B: JobBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx>,
{
    bundle: B,
    database: D,
    _marker: PhantomData<E>,
}

impl<E, B, D> JobService<E, B, D>
where
    B: JobBundle<E> + Sync + Send + 'static,
    D: for<'tx> DatabaseContextProvider<'tx> + Sync + Send + 'static,
    E: Sync + Send + 'static,
{
    pub fn new(bundle: B, database: D) -> Self {
        Self {
            bundle,
            database,
            _marker: Default::default(),
        }
    }

    /// Adds a job to a queue given the its definition.
    #[instrument(skip_all, fields(job_id = Empty), err)]
    pub async fn add_job(&self, job_request: CreateJobCommand) -> ApiResult<JobId> {
        let job_repo = self.bundle.job_repo();
        let queue_repo = self.bundle.queue_repo();

        // validates
        let valid_identity = JobIdentity::new(
            job_request.job_name,
            job_request.description,
            job_request.queue_name,
        );

        let valid_run_command = ContainerRunCommand::new(
            job_request.run_options,
            job_request.image_name,
            job_request.run_command,
            job_request.command_args,
        );

        let valid_options = JobOptions::new(
            job_request.schedule,
            job_request.max_retries,
            job_request.priority,
            job_request.max_concurrency,
            job_request.timeout_seconds,
        );

        let result = self
            .database
            .execute(|tx| async move {
                let queue_repo = queue_repo.clone();
                let job_repo = job_repo.clone();

                let valid_identity = valid_identity.clone();
                let valid_run_command = valid_run_command.clone();
                let valid_options = valid_options.clone();

                // instantiate job to enforce invariants
                let new_job_info = Job::new(valid_identity, valid_run_command, valid_options);

                let queue = err_ctx!(
                    queue_repo
                        .get_queue_by_name(tx, new_job_info.queue_name())
                        .await
                )?;

                let job_id = err_ctx!(job_repo.add_job(tx, &new_job_info, &queue.queue_id).await)?;
                Span::current().record("job_id", job_id.inner().to_string());

                err_ctx!(
                    queue_repo
                        .increment_used_capacity(tx, &queue.queue_id)
                        .await
                )?;

                Ok::<JobId, TransactionError>(job_id)
            })
            .await;

        let job_id = err_ctx!(result)?;
        info!(job_id = %job_id, "Job Added");

        Ok(job_id)
    }

    /// Returns a list of jobs that belong to the queue corresponding to the input `queue_id`.
    #[instrument(skip(self), err)]
    pub async fn get_jobs_by_queue_id(&self, queue_id: QueueId) -> ApiResult<Vec<JobItem>> {
        let job_repo = self.bundle.job_repo();
        let queue_repo = self.bundle.queue_repo();

        let result = self
            .database
            .execute(move |tx| {
                let job_repo = job_repo.clone();
                let queue_repo = queue_repo.clone();

                Box::pin(async move {
                    let queue = err_ctx!(queue_repo.get_queue_by_id(tx, &queue_id).await)?;

                    let jobs = err_ctx!(job_repo.get_jobs_by_queue_id(tx, &queue_id).await)?;

                    Ok::<(Vec<JobResponse>, String), TransactionError>((jobs, queue.queue_name))
                })
            })
            .await;

        let (jobs, queue_name) = err_ctx!(result)?;
        let job_items: Vec<JobItem> = jobs.iter().map(|j| j.to_job_item(&queue_name)).collect();

        Ok(job_items)
    }

    /// Returns a list of jobs that belong to the queue corresponding to the input `queue_name`.
    #[instrument(skip(self), err)]
    pub async fn get_jobs_by_queue_name(&self, queue_name: String) -> ApiResult<Vec<JobItem>> {
        let job_repo = self.bundle.job_repo();
        let queue_name_clone = queue_name.clone();

        let jobs = self
            .database
            .execute(move |tx| {
                let job_repo = job_repo.clone();
                let queue_name = queue_name.clone();

                Box::pin(async move {
                    let jobs = err_ctx!(job_repo.get_jobs_by_queue_name(tx, &queue_name).await)?;

                    Ok::<Vec<JobResponse>, TransactionError>(jobs)
                })
            })
            .await;

        let jobs = err_ctx!(jobs)?;
        let job_items: Vec<JobItem> = jobs
            .iter()
            .map(|j| j.to_job_item(&queue_name_clone))
            .collect();

        Ok(job_items)
    }

    #[instrument(skip(self), err)]
    pub async fn deactivate_job_by_id(&self, job_id: JobId) -> ApiResult<JobId> {
        let job_repo = self.bundle.job_repo();

        let result = self
            .database
            .execute(move |tx| {
                let job_repo = job_repo.clone();

                Box::pin(async move {
                    let job_id = err_ctx!(job_repo.deactivate_job_by_id(tx, &job_id).await)?;

                    Ok::<JobId, TransactionError>(job_id)
                })
            })
            .await;

        let job_id = err_ctx!(result)?;
        info!(job_id = %job_id, "Deactivated job");

        Ok(job_id)
    }

    #[instrument(skip(self), err)]
    pub async fn deactivate_job_by_name(&self, job_name: String) -> ApiResult<JobId> {
        let job_repo = self.bundle.job_repo();

        let result = self
            .database
            .execute(move |tx| {
                let job_repo = job_repo.clone();
                let job_name = job_name.clone();

                Box::pin(async move {
                    let job_id = err_ctx!(job_repo.deactivate_job_by_name(tx, &job_name).await)?;

                    Ok::<JobId, TransactionError>(job_id)
                })
            })
            .await;

        let job_id = err_ctx!(result)?;
        info!(job_id = %job_id, "Deactivated job");

        Ok(job_id)
    }

    #[instrument(skip(self), err)]
    pub async fn delete_job_by_id(&self, job_id: JobId) -> ApiResult<JobId> {
        let result = self
            .database
            .execute(move |tx| {
                let job_repo = self.bundle.job_repo();

                Box::pin(async move {
                    let job_id = err_ctx!(job_repo.delete_job_by_id(tx, &job_id).await)?;

                    Ok::<JobId, TransactionError>(job_id)
                })
            })
            .await;

        let job_id = err_ctx!(result)?;
        info!(job_id = %job_id, "Deleted job");

        Ok(job_id)
    }

    #[instrument(skip(self), err)]
    pub async fn delete_job_by_name(&self, job_name: String) -> ApiResult<JobId> {
        let result = self
            .database
            .execute(|tx| {
                let job_repo = self.bundle.job_repo();
                let job_name = job_name.clone();

                Box::pin(async move {
                    let job_id = err_ctx!(job_repo.delete_job_by_name(tx, &job_name).await)?;

                    Ok::<JobId, TransactionError>(job_id)
                })
            })
            .await;

        let job_id = err_ctx!(result)?;
        info!(job_id = %job_id, "Deleted job");

        Ok(job_id)
    }
}
