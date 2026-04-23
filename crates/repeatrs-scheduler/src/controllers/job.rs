//! Handles incoming gRPC requests.

use crate::error::{ApiError, ToStatusError};
use crate::services::job::JobService;
use repeatrs_bundles::JobBundle;
use repeatrs_domain::{JobId, NewJob, QueueId};
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

        let job_def = NewJob::try_from(req)
            .map_status_error(|| format!("Invalid job definition for job {}", &req.job_name))?;

        let result = self.service.add_job(job_def).await;

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
            return Status::invalid_argument("Missing queue id or name");
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
            return Status::invalid_argument("Missing job id or name");
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
            return Status::invalid_argument("Missing job id or name");
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

/// NewType used to instantiate a `NewJob`
struct NewJobRequest(NewJob);

/// Transfer `Job` type used for input validation at the controller layer.
impl TryFrom<AddJobRequest> for NewJobRequest {
    type Error = ApiError;

    //TODO: missing checks
    fn try_from(value: AddJobRequest) -> core::result::Result<Self, Self::Error> {
        let schedule = croner::Cron::from_str(&value.schedule)?;

        let job_name = value.job_name.replace(" ", "_");
        let queue_name = value.queue_name.replace(" ", "_");

        let args = if value.args.is_empty() {
            None::<String>
        } else {
            Some(value.args.join(" "))
        };

        let def = NewJob {
            job_name: value.job_name,
            description: value.description,
            schedule: schedule,
            options: value.options,
            image_name: value.image_name,
            command: value.command,
            args: value.args,
            max_retries: value.max_retries,
            status: value.status,
            priority: value.priority,
            queue_name: value.queue_name,
            max_concurrency: value.max_concurrency,
            timeout_seconds: value.timeout_seconds,
        };

        Ok(def)
    }
}
