use crate::AddJobArgs;
use crate::error::Result;

use super::JobService;
use repeatrs_proto::repeatrs::grpc_job_service_client::GrpcJobServiceClient;
use repeatrs_proto::repeatrs::{AddJobRequest, JobServiceResponse};
use tonic::transport::Channel;
use tonic::{Request, Response};

impl JobService {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AddJobArgs;
    use async_trait::async_trait;
    use repeatrs_proto::repeatrs::JobIdentifierRequest;
    use repeatrs_proto::repeatrs::grpc_job_service_client::GrpcJobServiceClient;
    use repeatrs_proto::repeatrs::grpc_job_service_server::{GrpcJobService, GrpcJobServiceServer};
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::Status;

    #[derive(Clone, Default)]
    struct MockGrpcCliService {}

    #[async_trait]
    impl GrpcJobService for MockGrpcCliService {
        async fn add_job(
            &self,
            request: Request<AddJobRequest>,
        ) -> std::result::Result<Response<JobServiceResponse>, Status> {
            let request = request.into_inner();

            assert_eq!(request.name, "test-job");
            assert_eq!(request.description, Some("A test job".to_string()));
            assert_eq!(request.schedule, "* * * * *");
            assert_eq!(request.image_name, "test/repo:latest");
            assert_eq!(request.command, Some("echo".to_string()));
            assert_eq!(request.args, vec!["hello".to_string()]);
            assert!(!request.no_retry);

            Ok(Response::new(JobServiceResponse {
                job_id: "mock-job-id".to_string(),
                success: true,
            }))
        }

        async fn deactivate_job(
            &self,
            _request: Request<JobIdentifierRequest>,
        ) -> std::result::Result<Response<JobServiceResponse>, Status> {
            unimplemented!()
        }

        async fn delete_job(
            &self,
            _request: Request<JobIdentifierRequest>,
        ) -> std::result::Result<Response<JobServiceResponse>, Status> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_handle_job_add() {
        let mock_service = MockGrpcCliService::default();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(GrpcJobServiceServer::new(mock_service))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        let mut client = GrpcJobServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let args = AddJobArgs {
            file: None,
            name: Some("test-job".to_string()),
            description: Some("A test job".to_string()),
            schedule: Some("* * * * *".to_string()),
            image_name: Some("test/repo:latest".to_string()),
            queue_name: Some("default".to_string()),
            no_retry: Some(false),
            options: None,
            command: Some("echo".to_string()),
            args: Some(vec!["hello".to_string()]),
            priority: None,
            max_concurrency: Some(100),
            timeout_seconds: Some(3600),
            dry_run: false,
        };

        let payload: AddJobRequest = args.try_into().unwrap();

        let result = add_job(payload, &mut client).await;
        assert!(result.is_ok());
    }
}
