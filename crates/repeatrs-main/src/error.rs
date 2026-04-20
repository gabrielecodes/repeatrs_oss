#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Internal error: {0}")]
    Internal(String),

    // --- Parsing & Casting ---
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Address parsing error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    // --- Transport ---
    #[error("gRPC status error: {0}")]
    TonicStatus(#[from] tonic::Status),

    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    // --- Nats ----
    #[error("Messaging system error: {0}")]
    Nats(#[from] repeatrs_nats::NatsError),
}
