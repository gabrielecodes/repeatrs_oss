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
