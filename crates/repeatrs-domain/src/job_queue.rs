use async_nats::jetstream::publish::PublishAck;
use serde::Serialize;

#[async_trait::async_trait]
pub trait JobQueueOperations: Send + Sync {
    type Err: core::error::Error + Send + Sync + 'static;

    async fn publish<T: Serialize + HasSubject + Sync>(
        &self,
        jobs: &[T],
    ) -> core::result::Result<Vec<Result<PublishAck, Self::Err>>, Self::Err>;
}

pub trait HasSubject: Sync {
    fn get_subject(&self) -> &str;
}
