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

pub use job::{Job, JobDefinition, JobId, JobOperations, JobStatus};
pub use job_queue::{HasSubject, JobQueueOperations};
pub use job_runs::{JobRunId, JobRunOperations, JobRunStatus};
pub use job_schedule_state::{JobRunInsert, JobScheduleState, JobScheduleStateOperations};
pub use queue::{Queue, QueueId, QueueOperations, QueueStatus};

use uuid::Uuid;

pub trait IsId: From<Uuid> + Into<Uuid> + Copy + PartialEq {}

pub type DomainResult<T> = core::result::Result<T, error::DomainError>;
