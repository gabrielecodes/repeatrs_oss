use crate::commands::job::JobIdentifierArgs;
use crate::error::Result;

use repeatrs_proto::repeatrs::JobIdentifierRequest;
use repeatrs_proto::repeatrs::grpc_job_service_client::GrpcJobServiceClient;
use tonic::transport::Channel;

pub async fn delete_job(
    identifier: JobIdentifierArgs,
    client: &mut GrpcJobServiceClient<Channel>,
) -> Result<()> {
    let request: JobIdentifierRequest = identifier.into();

    let response = client.delete_job(request).await?;

    let inner = response.get_ref();

    if inner.success {
        println!("Job '{}' deleted", inner.job_id);
    } else {
        println!("Failed deleting job");
    }

    Ok(())
}
