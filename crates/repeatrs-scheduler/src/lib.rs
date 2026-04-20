//! # repeatrs-scheduler
//!
//! The `repeatrs-scheduler` crate provides the core scheduling capabilities for the Repeatrs job execution system.
//! It is responsible for orchestrating the execution of scheduled jobs, managing their lifecycle,
//! and ensuring predictable, FIFO-based job processing.
//!
//! ## Key Responsibilities:
//!
//! - **Job Scheduling:** Identifies jobs that are due for execution based on defined schedules and priorities.
//! - **Job Lifecycle Management:** Loads, dispatches, and updates the status of jobs throughout their execution.
//! - **Database Interaction:** Manages job persistence and state using a PostgreSQL database.
//! - **Message Queueing (NATS JetStream):** Interacts with NATS JetStream for messaging between the scheduler and workers,
//!   including waking up workers and distributing job execution requests.
//! - **gRPC API:** Exposes a gRPC interface for external control and monitoring, allowing clients (e.g., a CLI)
//!   to interact with the scheduler.
//!
//! This crate aims to provide a robust and efficient foundation for managing a distributed job queue.

mod config;
mod controllers;
mod scheduler;
mod services;
mod utils;
#[macro_use]
mod error;

pub use config::{Config, config};
pub use controllers::job::JobController;
pub use controllers::queue::QueueController;
pub use services::{job::JobService, queue::QueueService, scheduling::SchedulingService};

pub use scheduler::Scheduler;
pub use utils::initialize_tracing;

pub type ServiceResult<T> = core::result::Result<T, error::ServiceError>;

// External Dependencies
// use core::net::SocketAddr;
// use repeatrs_proto::repeatrs::grpc_job_service_server::GrpcJobServiceServer;
// use repeatrs_proto::repeatrs::grpc_queue_service_server::GrpcQueueServiceServer;
// use sqlx::PgPool;
// use std::sync::Arc;
// use tokio::sync::{Notify, watch};
// use tonic::transport::Server;
// use tracing::info;

// /// Represents the running application, holding the handles to its core services.
// ///
// /// This struct is created by `AppService::build()` and encapsulates the spawned tasks
// /// for the scheduler and the gRPC server. Its primary role is to manage the application's
// /// lifecycle, particularly during graceful shutdown.
// pub struct AppService {
//     scheduler_handle: tokio::task::JoinHandle<ServiceResult<()>>,
//     server_handle: tokio::task::JoinHandle<ServiceResult<()>>,
// }

// impl AppService {
//     /// Waits for the application's services to shut down gracefully.
//     ///
//     /// This method should be called after `build()`. It awaits the completion of the gRPC server,
//     /// which is the primary driver of the shutdown sequence (e.g., via a Ctrl+C signal).
//     /// Once the server has stopped, it ensures the scheduler task is also cleaned up.
//     pub async fn start(self) -> Result<()> {
//         // cleanup
//         info!("Server stopped. Cleaning up tasks...");
//         match self.server_handle.await {
//             Ok(Ok(_)) => {}
//             Ok(Err(e)) => return Err(e),
//             Err(e) => return Err(Error::Internal(e.to_string())),
//         }

//         let _ = self
//             .scheduler_handle
//             .await
//             .map_err(|e| Error::Internal(e.to_string()));

//         info!("Service shut down gracefully.");

//         Ok(())
//     }

// /// Builds and initializes all application components, returning a runnable [`AppService`].
// ///
// /// This function is the main entry point for setting up the application. It handles:
// /// - Establishing database and NATS connections.
// /// - Spawning the [`Scheduler`] as a background task.
// /// - Spawning the gRPC server as a background task.
// ///
// /// If any part of the initialization fails, it will return an error, preventing the
// /// application from starting.
// pub async fn build() -> Result<Self> {
//     let pool = new_postgres_connection_pool()
//         .await
//         .expect("Could not connect to the database");

//     let nats_client = JetstreamClient::new().await?;

//     let wakeup = Arc::new(Notify::new());
//     let (shutdown_tx, shutdown_rx) = watch::channel(());

//     let scheduling_agent = SchedulingService::new(job_service, nats_client);

//     let scheduler = Scheduler::new(
//         pool.clone(),
//         scheduling_agent,
//         wakeup.clone(),
//         shutdown_rx.clone(),
//     );

//     let scheduler_handle = tokio::spawn(async move { scheduler.start().await });
//     // TODO: start failover

//     // start gRPC server
//     let server_handle = tokio::spawn(AppService::run_server(
//         pool,
//         wakeup.clone(),
//         shutdown_rx.clone(),
//         shutdown_tx.clone(),
//     ));

//     Ok(Self {
//         scheduler_handle,
//         server_handle,
//     })
// }

//     /// Starts the gRPC server initializing the controllers. The server is stopped when
//     /// `ctrl+c` is detected sending a termination signal to the main scheduler loop.
//     async fn run_server(
//         pool: PgPool,
//         wakeup: Arc<Notify>,
//         mut shutdown_rx: watch::Receiver<()>,
//         shutdown_tx: watch::Sender<()>,
//     ) -> Result<()> {
//         let job_repository = PgJobRepository::new(pool.clone());
//         let job_service = JobService::new(Arc::new(job_repository));
//         let job_controller = JobController::new(job_service, wakeup.clone());

//         let queue_repository = QueueRepository::new(pool.clone());
//         let queue_service = QueueService::new(queue_repository);
//         let queue_controller = QueueController::new(queue_service);

//         let addr: SocketAddr = config().scheduler_grpc_url().parse()?;

//         Server::builder()
//             .add_service(GrpcJobServiceServer::new(job_controller))
//             .add_service(GrpcQueueServiceServer::new(queue_controller))
//             .serve_with_shutdown(addr, async move {
//                 info!("Scheduler gRPC listening on {}", addr);

//                 tokio::select! {
//                     _ = tokio::signal::ctrl_c() => {
//                         info!("Received Ctrl+C, initiating shutdown");
//                         let _ = shutdown_tx.send(());
//                     }
//                     _ = shutdown_rx.changed() => {
//                         info!("Internal shutdown signal received");
//                     }
//                 }
//             })
//             .await
//             .map_err(Error::Transport)
//     }
// }
