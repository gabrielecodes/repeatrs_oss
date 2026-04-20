//! A high-velocity, "thin" table containing only pending work. It is organized
//! as a list of jobs partitioned by `queue_id`. While a [`crate::db::Queue`]
//! represents a container defining a queue with its basic characteristics, the
//! [`JobQueue`] is the actual content of the queue, representing a list of
//! jobs to be executed by Workers.
//!

mod queries;

use crate::jobs::JobId;

use chrono::{DateTime, Utc};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct JobQueueId(pub Uuid);

impl Display for JobQueueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for JobQueueId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

/// Represents the pool table
#[derive(Debug)]
pub struct JobQueue {
    /// The unique identifier of rhe pool.
    job_queue_id: JobQueueId,

    /// Identifier of the queued job.
    job_id: JobId,

    /// Queue name.
    name: Option<String>,

    // /// Job priority
    // priority: Option<i32>,

    // /// Times when the job is due to be executed
    // deadline: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl JobQueue {
    pub fn job_queue_id(&self) -> JobQueueId {
        self.job_queue_id
    }

    pub fn job_id(&self) -> JobId {
        self.job_id
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
