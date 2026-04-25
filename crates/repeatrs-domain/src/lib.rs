//! Domain types are basic entities that decouple database/repository types
//! from the service layer. The repository layer provides translation of its
//! own types to the domain types. The service layer works with domain objects.
//! The controller layer receives domain types from the services and converts
//! to gRPC compatible types (the DTOs).
//!
//! The characteristics of domain types are the following:
//! - Fields may wrap values in value objects (e.g. [`JobId`], [`JobStatus`]).
//! - Enforce domain invariants (disallow invalid states).
//! - Have no knowledge of the database or SQLx.
//!
//! Purpose:
//! - Database changes don’t break domain
//! - Domain invariants are preserved. Invalid data from DB cannot appear in the
//!   service layer. Rules specific to each entity are enforced. If the rule can be
//!   enforced by the entity itself independently from other entities, it belongs in
//!   the domain layer.
//! - Different query shapes are supported. Queries can return different types but
//!   map to the same domain type
//!

pub mod error;
mod job;
mod job_queue;
mod job_runs;
mod job_schedule_state;
mod queue;
mod worker;

use std::ops::Deref;

pub use error::ValidationError;
pub use job::{
    CommandArgs, ContainerOptions, ContainerRunCommand, ImageName, Job, JobId, JobIdentity,
    JobOperations, JobOptions, JobResponse, JobStatus, RunCmd, ToOwnedString,
};
pub use job_queue::{HasSubject, JobQueueOperations};
pub use job_runs::{ExitStatus, JobRun, JobRunId, JobRunOperations, JobRunStatus, NewJobRun};
pub use job_schedule_state::{JobScheduleState, JobScheduleStateOperations, NewJobScheduleState};
pub use queue::{Queue, QueueId, QueueOperations, QueueStatus};
pub use worker::WorkerId;

use uuid::Uuid;

pub trait IsId: From<Uuid> + Into<Uuid> + Copy + PartialEq {}

pub type DomainResult<T> = core::result::Result<T, error::DomainError>;

#[derive(Debug, Clone)]
pub struct SanitizedString(String);

impl SanitizedString {
    pub fn new(str: String) -> Result<Self, ValidationError> {
        let sanitized = Self::sanitize(str)?;
        Ok(Self(sanitized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn sanitize(mut val: String) -> Result<String, ValidationError> {
        if val
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '_')
        {
            if val.contains(' ') {
                val = val.replace(' ', "_");
            }
            Ok(val)
        } else {
            Err(ValidationError::InvalidCharacters(val))
        }
    }
}

impl Deref for SanitizedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
