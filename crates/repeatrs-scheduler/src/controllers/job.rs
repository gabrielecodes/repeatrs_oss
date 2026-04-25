//! Handles incoming gRPC requests.

use crate::error::{ApiError, ToStatusError};
use crate::services::job::JobService;

use croner::Cron;
use repeatrs_bundles::JobBundle;
use repeatrs_domain::{
    CommandArgs, ContainerOptions, ImageName, JobId, QueueId, RunCmd, SanitizedString,
    ValidationError,
};
use repeatrs_proto::repeatrs::grpc_job_service_server::GrpcJobService;
use repeatrs_proto::repeatrs::job_identifier_request::JobIdentifier;
use repeatrs_proto::repeatrs::queue_identifier_request::QueueIdentifier;
use repeatrs_proto::repeatrs::{
    AddJobRequest, JobIdentifierRequest, JobListResponse, JobServiceResponse,
    QueueIdentifierRequest,
};
use repeatrs_transaction::DatabaseContextProvider;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Notify;
use tonic::{Request, Response, Status};
use tracing::{Span, error, field::Empty};

pub struct JobController<E, B, D>
where
    B: JobBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    service: JobService<E, B, D>,
    wakeup: Arc<Notify>,
    _marker: PhantomData<E>,
}

impl<E, B, D> JobController<E, B, D>
where
    B: JobBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    pub fn new(service: JobService<E, B, D>, wakeup: Arc<Notify>) -> Self {
        Self {
            service,
            wakeup,
            _marker: Default::default(),
        }
    }
}

#[tonic::async_trait]
impl<E, B, D> GrpcJobService for JobController<E, B, D>
where
    B: JobBundle<E> + Send + Sync + 'static,
    D: for<'tx> DatabaseContextProvider<'tx, E> + Send + Sync + 'static,
    E: Sync + Send + 'static,
{
    #[tracing::instrument(name = "add_job", skip_all, err, fields(job_name = Empty, queue_name = Empty))]
    async fn add_job(
        &self,
        request: Request<AddJobRequest>,
    ) -> std::result::Result<Response<JobServiceResponse>, Status> {
        let req = request.into_inner();

        Span::current().record("job_name", &req.job_name);
        Span::current().record("queue_name", &req.queue_name);

        let job_request = CreateJobCommand::try_from(req).map_status_error("Invalid input")?;

        let result = self.service.add_job(job_request).await;

        match result {
            Ok(job_id) => {
                self.wakeup.notify_one();
                Ok(Response::new(JobServiceResponse {
                    job_id: job_id.to_string(),
                    success: true,
                }))
            }
            Err(e) => {
                error!(error = %e.to_string(), "Failed adding job.");
                Err(Status::internal(e.to_string()))
            }
        }
    }

    #[tracing::instrument(name = "list_jobs", skip(self), err)]
    async fn list_jobs(
        &self,
        request: Request<QueueIdentifierRequest>,
    ) -> std::result::Result<Response<JobListResponse>, Status> {
        let identifier = request.into_inner();

        let queue_identifier = identifier.queue_identifier.ok_or_else(|| {
            tracing::warn!("Missing queue identifier");
            Status::invalid_argument("Missing queue id or name")
        })?;

        let result = match queue_identifier {
            QueueIdentifier::QueueId(recv_id) => {
                let queue_id: QueueId = recv_id
                    .parse()
                    .map_err(|_| Status::invalid_argument("Invalid queue id"))?;

                self.service.get_jobs_by_queue_id(queue_id).await
            }
            QueueIdentifier::QueueName(queue_name) => {
                self.service.get_jobs_by_queue_name(queue_name).await
            }
        };

        match result {
            Ok(jobs) => Ok(Response::new(JobListResponse { jobs })),
            Err(e) => {
                error!(error = %e.to_string(), "Failed adding job.");
                Err(Status::internal(e.to_string()))
            }
        }
    }

    #[tracing::instrument(name = "deactivate_job", skip(self), err)]
    async fn deactivate_job(
        &self,
        request: Request<JobIdentifierRequest>,
    ) -> std::result::Result<Response<JobServiceResponse>, Status> {
        let identifier = request.into_inner();

        let job_identifier = identifier.job_identifier.ok_or_else(|| {
            tracing::warn!("Missing job identifier");
            Status::invalid_argument("Missing job id or name")
        })?;

        let result = match job_identifier {
            JobIdentifier::JobId(recv_id) => {
                let job_id: JobId = recv_id
                    .parse()
                    .map_err(|_| Status::invalid_argument("Invalid job id"))?;

                self.service.deactivate_job_by_id(job_id).await
            }
            JobIdentifier::JobName(job_name) => self.service.deactivate_job_by_name(job_name).await,
        };

        match result {
            Ok(job_id) => {
                self.wakeup.notify_one();
                Ok(Response::new(JobServiceResponse {
                    job_id: job_id.inner().to_string(),
                    success: true,
                }))
            }
            Err(e) => {
                error!("Failed deactivating job: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }

    #[tracing::instrument(name = "delete_job", skip(self), err)]
    async fn delete_job(
        &self,
        request: Request<JobIdentifierRequest>,
    ) -> std::result::Result<Response<JobServiceResponse>, Status> {
        let identifier = request.into_inner();

        let job_identifier = identifier.job_identifier.ok_or_else(|| {
            tracing::warn!("Missing job identifier");
            Status::invalid_argument("Missing job id or name")
        })?;

        let result = match job_identifier {
            JobIdentifier::JobId(recv_id) => {
                let job_id: JobId = recv_id
                    .parse()
                    .map_err(|_| Status::invalid_argument("Invalid job id"))?;

                self.service.delete_job_by_id(job_id).await
            }
            JobIdentifier::JobName(job_name) => self.service.delete_job_by_name(job_name).await,
        };

        match result {
            Ok(job_id) => {
                self.wakeup.notify_one();
                Ok(Response::new(JobServiceResponse {
                    job_id: job_id.inner().to_string(),
                    success: true,
                }))
            }
            Err(e) => {
                error!("Failed deleting job: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateJobCommand {
    /// Unique job name
    pub job_name: SanitizedString,

    /// Unique queue identifier
    pub queue_name: SanitizedString,

    /// The job description
    pub description: Option<String>,

    /// schedule of the cronjob or execution time
    pub schedule: Cron,

    /// Options for running the container
    pub run_options: ContainerOptions,

    /// Image name as <optional_registry>/<image_name>:tag
    pub image_name: ImageName,

    /// The command to run the container
    pub run_command: RunCmd,

    /// Arguments for the command
    pub command_args: CommandArgs,

    /// Retry the job if last execution failed
    pub max_retries: Option<i32>,

    /// Job priority
    pub priority: Option<i32>,

    /// Maximum number of identical jobs running concurrently
    pub max_concurrency: Option<i32>,

    /// A hard limit on the duration of the job, after which the job is terminated. Default: 2 hours
    pub timeout_seconds: Option<i64>,
}

impl TryFrom<AddJobRequest> for CreateJobCommand {
    type Error = ApiError;

    fn try_from(value: AddJobRequest) -> core::result::Result<Self, Self::Error> {
        let schedule = croner::Cron::from_str(&value.schedule).map_err(|e| {
            let err = ValidationError::InvalidCron(e);
            ApiError::Validation(err)
        })?;

        let job_name = SanitizedString::new(value.job_name)?;
        let queue_name = SanitizedString::new(value.queue_name)?;

        let input_options = value.options.unwrap_or_default();
        let run_options: ContainerOptions = input_options.try_into()?;

        let image_name: ImageName = value.image_name.try_into()?;

        let run_command = RunCmd::new(value.command);

        let input_args = value.args.unwrap_or_default();
        let command_args: CommandArgs = input_args.try_into()?;

        let new_job = CreateJobCommand {
            job_name,
            queue_name,
            description: value.description,
            schedule,
            run_options,
            image_name,
            run_command,
            command_args,
            max_retries: value.max_retries,
            priority: value.priority,
            max_concurrency: value.max_concurrency,
            timeout_seconds: value.timeout_seconds,
        };

        Ok(new_job)
    }
}
