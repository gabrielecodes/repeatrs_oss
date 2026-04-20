//! Errors raiesed by breaking business logic rules
//!
//! Protocol & Input Errors:
//! - Malformed Requests: Field user_id is missing
//! - Type Mismatches: Failed to parse into UUID or Cron
//! - Authentication/Authorization: Missing permission, malformed token

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Entity not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Database Error")]
    Database,

    #[error("Messaging system failure")]
    Queue,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal system error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}
