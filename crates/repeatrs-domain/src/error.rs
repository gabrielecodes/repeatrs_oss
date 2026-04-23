//! Errors raiesed by breaking business logic rules
//!
//! Protocol & Input Errors:
//! - Malformed Requests: Field user_id is missing
//! - Type Mismatches: Failed to parse into UUID or Cron
//! - Authentication/Authorization: Missing permission, malformed token

use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Validation failed: {0}")]
    Validation(ValidationError),

    #[error("Database Error")]
    Database,

    #[error("Internal system error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    InvalidImageName,
    InvalidContainerOptions,
    InvalidRunCommand,
    InvalidRunArguments,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            ValidationError::InvalidImageName => "Invalid image name",
            ValidationError::InvalidContainerOptions => "Invalid container options",
            ValidationError::InvalidRunCommand => "Invalid run command",
            ValidationError::InvalidRunArguments => "Invalid command arguments",
        };

        write!(f, "{}", err)
    }
}
