//! Errors raiesed by breaking business logic rules
//!
//! Protocol & Input Errors:
//! - Malformed Requests: Field user_id is missing
//! - Type Mismatches: Failed to parse into UUID or Cron
//! - Authentication/Authorization: Missing permission, malformed token

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Internal system error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),

    // Database errors
    #[error("Record not found")]
    NotFound,

    #[error("Unexpected database error")]
    Database,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Error parsing UUID: {0}")]
    UuidValidation(#[from] uuid::Error),

    #[error("invalid image name: {0}")]
    InvalidImageName(String),

    #[error(transparent)]
    InvalidContainerOption(ContainerOptionsError),

    #[error("invalid run command: {0}")]
    InvalidRunCommand(String),

    #[error("invalid command argument: {0}")]
    InvalidRunArguments(String),

    #[error("String is not alphanumeric: {0}")]
    InvalidCharacters(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCron(#[from] croner::errors::CronError),
}

#[derive(Debug, thiserror::Error)]
pub enum ContainerOptionsError {
    #[error("Malformed argument '{0}': expected --key=value")]
    MalformedArgument(String),

    #[error("Missing value for argument '{0}'")]
    MissingValue(String),

    #[error("Unexpected positional argument '{0}'")]
    UnexpectedPositional(String),
}
