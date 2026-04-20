use std::{fmt::Display, ops::Deref, str::FromStr};

use crate::IsId;

use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use repeatrs_proto::repeatrs::QueueItem;
use serde::Serialize;
use uuid::Uuid;

/// Status of the queue. `ACTIVE` queues are eligible to receive new elements
/// while `INACTIVE` are not. Attemps to insert new elements in an inactive
/// queue results in an error.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum QueueStatus {
    /// The queue accepts new elements
    #[default]
    Active,

    /// The queue does not accept new elements
    Inactive,
}

impl Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Active => "ACTIVE",
            Self::Inactive => "INACTIVE",
        };
        f.write_str(s)
    }
}

impl FromStr for QueueStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ACTIVE" => Ok(Self::Active),
            "INACTIVE" => Ok(Self::Inactive),
            _ => Err("Job status not understood.".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Eq)]
pub struct QueueId(pub Uuid);
impl IsId for QueueId {}

impl From<Uuid> for QueueId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl Deref for QueueId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<QueueId> for Uuid {
    fn from(job_id: QueueId) -> Self {
        job_id.0
    }
}

impl FromStr for QueueId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(QueueId(Uuid::from_str(s)?))
    }
}

impl std::fmt::Display for QueueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl QueueId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn inner(self) -> Uuid {
        self.0
    }
}

pub struct Queue {
    /// Unique identifier for this queue
    pub queue_id: QueueId,

    /// The name of this queue
    pub queue_name: String,

    /// Status of the queue
    pub status: QueueStatus,

    /// Maximum number of jobs for this queue
    pub capacity: i32,

    /// Current number of jobs in this queue
    pub used_capacity: i32,

    /// Timestamp of queue creation
    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
}

impl Queue {
    pub fn to_queue_item(&self) -> QueueItem {
        let created_at = Timestamp {
            seconds: self.created_at.timestamp(),
            nanos: self.created_at.timestamp_subsec_nanos() as i32,
        };

        QueueItem {
            queue_name: self.queue_name.to_owned(),
            status: self.status.to_string(),
            capacity: self.capacity,
            used_capacity: self.used_capacity,
            created_at: Some(created_at),
        }
    }
}

pub trait QueueOperations<E>: Send + Sync + 'static {
    type Err: core::error::Error;

    fn add_queue(
        &self,
        tx: &mut E,
        queue_name: &str,
    ) -> impl std::future::Future<Output = Result<QueueId, Self::Err>> + Send;

    fn get_queues(
        &self,
        tx: &mut E,
    ) -> impl std::future::Future<Output = Result<Vec<Queue>, Self::Err>> + Send;

    fn get_queues_by_id(
        &self,
        tx: &mut E,
        queue_names: &[QueueId],
    ) -> impl std::future::Future<Output = Result<Vec<Queue>, Self::Err>> + Send;

    fn get_queue_by_id(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<Queue, Self::Err>> + Send;

    fn get_queue_by_name(
        &self,
        tx: &mut E,
        queue_name: &str,
    ) -> impl std::future::Future<Output = Result<Queue, Self::Err>> + Send;

    fn increment_used_capacity(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;

    fn decrement_used_capacity(
        &self,
        tx: &mut E,
        queue_id: &QueueId,
    ) -> impl std::future::Future<Output = Result<(), Self::Err>> + Send;
}
