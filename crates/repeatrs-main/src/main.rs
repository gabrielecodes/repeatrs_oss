mod error;

use core::net::SocketAddr;
use error::AppError;
use repeatrs_bundles::{PgJobBundle, PgJobSchedulerBundle, PgQueueBundle};
use repeatrs_proto::repeatrs::grpc_job_service_server::GrpcJobServiceServer;
use repeatrs_proto::repeatrs::grpc_queue_service_server::GrpcQueueServiceServer;
use repeatrs_scheduler::{
    JobController, JobService, QueueController, QueueService, Scheduler, SchedulingService, config,
};
use repeatrs_transaction::DatabaseContext;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tokio::sync::{Notify, watch};
use tokio::time::Duration;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let pool = new_postgres_connection_pool()
        .await
        .expect("Could not connect to the database");

    let nats_config = repeatrs_nats::nats_config();

    let wakeup = Arc::new(Notify::new());
    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let database_context = DatabaseContext::new(pool.clone());

    let job_bundle = PgJobBundle;
    let job_service = JobService::new(job_bundle, database_context.clone());
    let job_controller = JobController::new(job_service, wakeup.clone());

    let queue_bundle = PgQueueBundle;
    let queue_service = QueueService::new(queue_bundle, database_context.clone());
    let queue_controller = QueueController::new(queue_service);

    let scheduler_bundle = PgJobSchedulerBundle;
    let scheduling_agent = SchedulingService::new(scheduler_bundle, database_context.clone());
    let scheduler = Scheduler::new(scheduling_agent, wakeup.clone(), shutdown_rx.clone());

    let scheduler_handle = tokio::spawn(async move {
        scheduler.start().await;
    });

    let addr: SocketAddr = config().scheduler_grpc_url().parse()?;

    let server_handle = tokio::spawn(async {
        Server::builder()
            .add_service(GrpcJobServiceServer::new(job_controller))
            .add_service(GrpcQueueServiceServer::new(queue_controller))
            .serve_with_shutdown(addr, async move {
                info!("Scheduler gRPC listening on {}", addr);

                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        info!("Received Ctrl+C, initiating shutdown");
                        let _ = shutdown_tx.send(());
                    }
                    _ = shutdown_rx.changed() => {
                        info!("Internal shutdown signal received");
                    }
                }
            })
            .await
            .map_err(AppError::Transport)
    });

    // cleanup
    info!("Server stopped. Cleaning up tasks...");
    match server_handle.await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(e),
        Err(e) => return Err(AppError::Internal(e.to_string())),
    }

    let _ = scheduler_handle
        .await
        .map_err(|e| AppError::Internal(e.to_string()));

    info!("Service shut down gracefully.");

    Ok(())
}

/// Initializes a connection pool to the database at the given path.
pub async fn new_postgres_connection_pool() -> Result<PgPool, AppError> {
    let pg_config = config().postgres();

    let max_conn = pg_config.max_connections();
    let acquire_timeout_secs = pg_config.acquire_timeout_secs();
    let idle_timeout_secs = pg_config.idle_timeout_secs();
    let max_lifetime_secs = pg_config.max_lifetime_secs();
    let url = pg_config.url();

    let pool = PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs as u64))
        .idle_timeout(Duration::from_secs(idle_timeout_secs as u64))
        .max_lifetime(Duration::from_secs(max_lifetime_secs as u64))
        .connect(url)
        .await;

    pool.map_err(|_| AppError::Internal("Could not obtain a connection pool".to_string()))
}
