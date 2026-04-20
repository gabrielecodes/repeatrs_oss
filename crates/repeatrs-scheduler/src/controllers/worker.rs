//! Handles incoming gRPC requests from workers.
//!
//! The Worker calls subscribe_worker from the [`GrpcWorkerService`] instantiating
//! an mpsc channel, handing over the receiver to Tonic runtime and the sender to
//! the scheduler.
//!
//! The scheduler sends wake up messages to the workers when jobs are ready to be
//! executed.
//!
//! The next time the scheduler tries to tx.send(), it gets a SendError. A process
//! of cleaning up dead workers can then be initiated.
//!
//! Uses the trait [`GrpcWorkerService`] to define the transport layer.

use repeatrs_proto::repeatrs::grpc_worker_service_server::GrpcWorkerService;
use repeatrs_proto::repeatrs::{SchedulerSignal, WorkerReadyMessage, WorkerReadyMessage};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

use crate::api::services::WorkerService;

// Tonic gRPC requires a type that implements `Stream`. Tonic consumes the stream
// and sends the message over the gRPC channel. Moreover, We need a mpsc channel so
// that the scheduler can use the tx side of the channel to send the wake up to the
// worker, but mspc::channel does not implement Stream so we wrap it in ReceiverStream.
// Tonic --> Requires type that implements Stream
// mpsc Channel --> Wrapped in ReceiverStream --> Implements Stream
type ResponseStream = ReceiverStream<Result<SchedulerSignal, tonic::Status>>;

pub struct WorkerController {
    worker_registry: Arc<WorkerRegistry>,
    worker_service: WorkerService,
}

impl WorkerController {
    pub fn new(worker_registry: Arc<WorkerRegistry>, worker_service: WorkerService) -> Self {
        Self {
            worker_registry,
            worker_service,
        }
    }
}

#[tonic::async_trait]
impl GrpcWorkerService for WorkerController {
    type SubscribeWorkerStream = ResponseStream;

    #[tracing::instrument(level = "debug", name="grpc.subscribe_worker" skip_all, err)]
    async fn subscribe_worker(
        &self,
        request: tonic::Request<WorkerReadyMessage>,
    ) -> std::result::Result<tonic::Response<Self::SubscribeWorkerStream>, tonic::Status> {
        let msg = request.as_ref();

        let inner = request.into_inner();
        let worker_id = Uuid::parse_str(&inner.worker_id)
            .map_err(|_| Status::invalid_argument("Failed parsing worker id"))?;

        let queue_id = self.db.get_worker_queue(worker_id).await?;

        let (tx, rx) = mpsc::channel(128);

        self.registry.register(queue_id, worker_id, tx);

        let _guard = UnregisterGuard {
            registry: self.registry.clone(),
            queue_id,
            worker_id,
        };

        // tonic spawns a background task that sits there "polling" that receiver.
        let stream = ReceiverStream::new(rx);
        Ok(tonic::Response::new(stream))
    }

    async fn request_job_dispatch(
        &self,
        request: tonic::Request<super::WorkerResources>,
    ) -> std::result::Result<tonic::Response<super::JobPayload>, tonic::Status> {
        // use self.worker_service to get the job payload to send to the workers
        unimplemented!()
    }
}
