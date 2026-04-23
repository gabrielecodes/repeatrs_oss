use crate::services::queue::QueueService;
use repeatrs_bundles::QueueBundle;
use repeatrs_proto::repeatrs::grpc_queue_service_server::GrpcQueueService;
use repeatrs_proto::repeatrs::{Empty, QueueListResponse};
use repeatrs_transaction::DatabaseContextProvider;
use std::marker::PhantomData;
use tonic::{Request, Response, Status};
use tracing::error;

pub struct QueueController<E, B, D>
where
    B: QueueBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    service: QueueService<E, B, D>,
    _marker: PhantomData<E>,
}

impl<E, B, D> QueueController<E, B, D>
where
    B: QueueBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    pub fn new(service: QueueService<E, B, D>) -> Self {
        Self {
            service,
            _marker: Default::default(),
        }
    }
}

#[tonic::async_trait]
impl<E, B, D> GrpcQueueService for QueueController<E, B, D>
where
    B: QueueBundle<E> + Send + Sync + 'static,
    D: for<'tx> DatabaseContextProvider<'tx, E> + Send + Sync + 'static,
    E: Sync + Send + 'static,
{
    #[tracing::instrument(skip(self))]
    async fn list_queues(
        &self,
        _request: Request<Empty>,
    ) -> std::result::Result<tonic::Response<QueueListResponse>, tonic::Status> {
        let response = self.service.get_queues().await;

        match response {
            Ok(queues) => Ok(Response::new(QueueListResponse { queues })),
            Err(e) => {
                error!(error = %e.to_string(), "Failed fetching queues");
                Err(Status::internal(e.to_string()))
            }
        }
    }
}

// /// Represents the information needed to validate and instantiate a [`NewQueue`]
// pub struct NewQueueInput(NewQueue);

// impl NewQueueInput {
//     pub fn new(job_info: NewQueue) -> Self {
//         Self(job_info)
//     }

//     /// Consumes this object and returns its inner value
//     pub fn to_inner(self) -> NewQueue {
//         self.0
//     }
// }

// impl TryFrom<AddQuRequest> for NewQueueInput {
//     type Error = ApiError;

//     //TODO: missing checks
//     fn try_from(value: AddJobRequest) -> core::result::Result<Self, Self::Error> {
//         let schedule = croner::Cron::from_str(&value.schedule)?;

//         let job_name = value.job_name.replace(" ", "_");
//         let queue_name = value.queue_name.replace(" ", "_");

//         let args = if value.args.is_empty() {
//             None::<String>
//         } else {
//             Some(value.args.join(" "))
//         };

//         let new_job = NewQueue {
//             job_name: job_name,
//             description: value.description,
//             schedule,
//             options: value.options,
//             image_name: value.image_name,
//             command: value.command,
//             args,
//             max_retries: value.max_retries,
//             priority: value.priority,
//             queue_name: queue_name,
//             max_concurrency: value.max_concurrency,
//             timeout_seconds: value.timeout_seconds,
//         };

//         Ok(NewQueueInput::new(new_job))
//     }
// }
