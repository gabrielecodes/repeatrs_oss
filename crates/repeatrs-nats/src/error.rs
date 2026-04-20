use async_nats::{ConnectErrorKind, jetstream::context::CreateStreamErrorKind};
use config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum NatsError {
    #[error("Internal error: {0}")]
    Internal(String),

    // --- NATS ----
    #[error("Invalid Address: {0}")]
    InvalidAddress(std::io::Error),

    #[error("Error establishing a connection to NATS: {0}")]
    NatsConnection(#[from] async_nats::error::Error<ConnectErrorKind>),

    #[error("Error creating NATS stream: {0}")]
    NatsCreateStream(#[from] async_nats::error::Error<CreateStreamErrorKind>),

    #[error("Error publishing message: {0}")]
    NatsPublish(#[from] async_nats::error::Error<async_nats::jetstream::context::PublishErrorKind>),

    #[error("Error retrieving subjects: {0}")]
    NatsSubjects(String),

    // --- Parsing & Casting ---
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // --- Transport ---
    #[error("gRPC status error: {0}")]
    TonicStatus(#[from] tonic::Status),

    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    // --- Config ---
    #[error("{0}")]
    Config(#[from] ConfigError),
}
