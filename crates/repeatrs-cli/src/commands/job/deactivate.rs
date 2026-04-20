use crate::commands::job::JobIdentifierArgs;
use crate::error::Result;

use repeatrs_proto::repeatrs::JobIdentifierRequest;
use repeatrs_proto::repeatrs::grpc_job_service_client::GrpcJobServiceClient;
use tonic::transport::Channel;

pub async fn deactivate_job(
    identifier: JobIdentifierArgs,
    client: &mut GrpcJobServiceClient<Channel>,
) -> Result<()> {
    let request: JobIdentifierRequest = identifier.into();

    let response = client.deactivate_job(request).await?;

    let inner = response.get_ref();

    if inner.success {
        println!("{}", inner.job_id)
    } else {
        println!("Failed deactivating job")
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use repeatrs_proto::repeatrs::grpc_job_service_client::GrpcJobServiceClient;
    use repeatrs_proto::repeatrs::grpc_job_service_server::{GrpcJobService, GrpcJobServiceServer};
    use repeatrs_proto::repeatrs::{AddJobRequest, JobIdentifierRequest, JobServiceResponse};
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};
    use uuid::Uuid;

    use super::*;

    #[derive(Clone, Default)]
    struct MockGrpcJobService;

    #[async_trait::async_trait]
    impl GrpcJobService for MockGrpcJobService {
        async fn deactivate_job(
            &self,
            _request: Request<JobIdentifierRequest>,
        ) -> std::result::Result<Response<JobServiceResponse>, Status> {
            println!("Server: job deactivated");
            Ok(Response::new(JobServiceResponse {
                job_id: "mock-job-id".to_string(),
                success: true,
            }))
        }

        async fn add_job(
            &self,
            _request: Request<AddJobRequest>,
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

    async fn start_mock_server(listener: TcpListener, service: impl GrpcJobService) {
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(GrpcJobServiceServer::new(service))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
    }

    #[tokio::test]
    async fn test_deactivate_job_by_id() {
        let service = MockGrpcJobService;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        start_mock_server(listener, service).await;

        let mut client = GrpcJobServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let uid: Uuid = "019c398e-4f39-79cd-b85f-a2ed4d79273f".parse().unwrap();
        let ident = JobIdentifierArgs {
            id: Some(uid),
            name: None,
        };

        let res = deactivate_job(ident, &mut client).await;
        assert!(res.is_ok())
    }
}
