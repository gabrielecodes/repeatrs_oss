//! The service layer error modes correspond to failures to conform to
//! busness logic rules such as:
//!
//! Logic-Based Rules:
//! - State Transitions (e.g. Failed jobs cannot become completed)
//! - Calculations (e.g. Totals exceeded)
//! - Cross-Entity Requirements (e.g. A Job must belong to a queue)
//! - Idempotency (e..g Job already running, Job already exists/not found)
//!
//! Domain-Specific Validation
//! - Custom Formats
//! - Time Rules
//!

use config::ConfigError;
use repeatrs_domain::error::DomainError;
use repeatrs_transaction::TransactionError;
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServiceError {
    #[error("Internal error")]
    Error {
        #[source]
        source: Box<dyn std::error::Error>,
    },

    #[error("{0}")]
    SchedulingError(String),

    // --- Config ---
    #[error("{0}")]
    Config(#[from] ConfigError),

    // --- Database ---
    #[error("{0}")]
    Transaction(#[from] TransactionError),

    // --- Parsing & Casting ---
    #[error("Error parsing UUID: {0}")]
    Uuid(#[from] uuid::Error),

    // --- Misc ---
    #[error("{0}")]
    Input(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait ToStatusError<T> {
    fn map_status_error(self, msg: &'static str) -> Result<T, tonic::Status>;
}

impl<T> ToStatusError<T> for Result<T, DomainError> {
    fn map_status_error(self, msg: &'static str) -> Result<T, tonic::Status> {
        //TODO: match into different status errors giving context
        // self.map_err(|e| match e { })
        Err(tonic::Status::invalid_argument(msg))
    }
}

// impl From<DomainError> for ServiceError {
//     fn from(error: DomainError) -> Self {
//         match error {
//             DomainError::NotFound => tonic::Status::not_found("Resource not found"),
//             DomainError::Validation(msg) => tonic::Status::invalid_argument(msg),
//             DomainError::Database | DomainError::Queue => tonic::Status::internal("Data error"),
//             DomainError::Internal(e) => tonic::Status::internal(e.to_string()),
//             _ => tonic::Status::unknown("An unexpected error occurred"),
//         }
//     }
// }

pub trait ToServiceError<T> {
    fn map_transaction_error(self, line: u32, file: &'static str) -> Result<T, TransactionError>;
}

impl<T, E> ToServiceError<T> for Result<T, E>
where
    E: std::fmt::Display + 'static,
{
    fn map_transaction_error(self, line: u32, file: &'static str) -> Result<T, TransactionError> {
        self.map_err(|_| TransactionError::Error(line, file))
    }
}

#[macro_export]
macro_rules! err_ctx {
    ($result:expr) => {
        $result.map_transaction_error(line!(), file!())
    };
}
