use crate::config::Config;
use crate::{NatsError, Result};
use async_nats::jetstream::publish::PublishAck;
use async_nats::jetstream::{
    self, Context,
    stream::{self, RetentionPolicy},
};
use futures::future::join_all;
use repeatrs_domain::{HasSubject, Job, JobQueueOperations};
use serde::Serialize;
use tokio::time::Duration;

pub struct JetstreamContextBuilder {
    config: Config,
    user: Option<String>,
    password: Option<String>,
    // Possible to inject client for testing
    client: Option<async_nats::Client>,
}

impl JetstreamContextBuilder {
    pub fn with_config(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            user: None,
            password: None,
            client: None,
        })
    }

    pub fn with_user_and_password(mut self, user: String, password: String) -> Self {
        self.password = Some(password);
        self.user = Some(user);

        self
    }

    pub async fn build(self) -> Result<Context> {
        let mut options = async_nats::ConnectOptions::new();
        if let (Some(user), Some(pwd)) = (self.user, self.password) {
            // Assuming username is part of the address or provided separately
            options = options.user_and_password(user, pwd)
        }

        let client = match self.client {
            Some(c) => c,
            None => async_nats::connect_with_options(self.config.address(), options).await?,
        };
        let context = jetstream::new(client);

        let prefix = self.config.job_queues_prefix();

        let stream_config = stream::Config {
            name: prefix.to_string(),
            subjects: vec![format!(
                "{}.{}.>",
                self.config.environment().to_uppercase(),
                prefix
            )],
            description: Some("job queues".to_string()),
            retention: RetentionPolicy::WorkQueue,
            allow_direct: true,
            max_consumers: self.config.max_consumers(),
            max_messages: self.config.max_messages(),
            max_bytes: self.config.max_bytes(),
            discard: jetstream::stream::DiscardPolicy::New,
            duplicate_window: Duration::from_secs(self.config.duplicate_window_seconds()),
            ..Default::default()
        };

        context
            .get_or_create_stream(stream_config)
            .await
            .map_err(|e| {
                tracing::error!("Failed to initialize NATS stream: {}", e);
                e
            })?;

        Ok(context)
    }
}

/// Handle to a NATS context with associated streams.
#[derive(Debug, Clone)]
pub struct NatsJobQueueRepository<'a> {
    context: Context,
    environment: &'a str,
    prefix: &'a str,
}

impl<'a> NatsJobQueueRepository<'a> {
    pub fn new(context: Context, environment: &'a str, prefix: &'a str) -> Self {
        Self {
            context,
            environment,
            prefix,
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    fn get_subject(&self, name: &str) -> String {
        format!("{}.{}.{}", self.environment, self.prefix, name)
    }
}

#[async_trait::async_trait]
impl<'a> JobQueueOperations for NatsJobQueueRepository<'a> {
    type Err = NatsError;

    async fn publish<T>(&self, jobs: &[T]) -> Result<Vec<Result<PublishAck>>>
    where
        T: Serialize + HasSubject + Sync,
    {
        let mut msg_futures = Vec::with_capacity(jobs.len());

        for job in jobs {
            let mut headers = async_nats::HeaderMap::new();
            headers.insert("Nats-Msg-Id", uuid::Uuid::new_v4().to_string());
            headers.insert("X-Source-App", "repeatrs");

            let subject = self.get_subject(job.get_subject());

            let payload = serde_json::to_vec(&job)?;

            let ack_future = self
                .context
                .publish_with_headers(subject.clone(), headers, payload.into())
                .await?;

            msg_futures.push(ack_future);
        }

        // join_all turns Vec<Future<Result<Ack>>> into Future<Vec<Result<Ack>>>
        let results = join_all(
            msg_futures
                .into_iter()
                .map(|f| async move { f.await.map_err(NatsError::from) }),
        )
        .await;

        Ok(results)
    }
}

pub struct PublishableJob {
    payload: Job,
    subject: String,
}

impl PublishableJob {
    pub fn new(job: Job, subject: &str) -> Self {
        Self {
            payload: job,
            subject: subject.to_string(),
        }
    }
}

impl HasSubject for PublishableJob {
    fn get_subject(&self) -> &str {
        &self.subject
    }
}
