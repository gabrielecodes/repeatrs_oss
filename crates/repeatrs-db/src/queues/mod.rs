//! A queue is a container for the jobs that are waiting to be executed.
//! Workers assigned to a queue consume it and execute the jobs until
//! the queue is empty. If there are no resources available among the
//! workers serving a specific queue, the jobs wait in line until resources
//! become available.

mod queries;

pub(crate) use queries::*;

use crate::{DbResult, error::DbError};
use chrono::{DateTime, Utc};
use repeatrs_domain::{Queue, QueueId, QueueOperations, QueueStatus};
use sqlx::{Postgres, Transaction, Type};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgQueueRepository;

impl<'e> QueueOperations<Transaction<'e, Postgres>> for PgQueueRepository {
    type Err = DbError;

    async fn add_queue(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_name: &str,
    ) -> DbResult<QueueId> {
        let queue_id = add_queue(&mut **tx, queue_name).await?;
        Ok(queue_id.into())
    }

    async fn get_queues(&self, tx: &mut Transaction<'e, Postgres>) -> DbResult<Vec<Queue>> {
        let queues = get_queues(&mut **tx).await?;

        let queues: Vec<Queue> = queues.into_iter().map(|q| q.into()).collect();

        Ok(queues)
    }

    async fn get_queues_by_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_ids: &[QueueId],
    ) -> DbResult<Vec<Queue>> {
        let queue_ids: Vec<DbQueueId> = queue_ids.iter().map(DbQueueId::from).collect();

        let queue_rows = get_queues_by_id(&mut **tx, &queue_ids).await?;

        let queues: Vec<Queue> = queue_rows.into_iter().map(|q| q.into()).collect();

        Ok(queues)
    }

    async fn get_queue_by_id(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_id: &QueueId,
    ) -> DbResult<Queue> {
        let queue_row = get_queue_by_id(&mut **tx, &queue_id.into()).await?;

        Ok(queue_row.into())
    }

    async fn get_queue_by_name(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_name: &str,
    ) -> DbResult<Queue> {
        let queue = get_queue_by_name(&mut **tx, queue_name).await?;

        Ok(queue.into())
    }

    async fn increment_used_capacity(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_id: &QueueId,
    ) -> DbResult<()> {
        increment_used_capacity(&mut **tx, &queue_id.into()).await?;
        Ok(())
    }

    async fn decrement_used_capacity(
        &self,
        tx: &mut Transaction<'e, Postgres>,
        queue_id: &QueueId,
    ) -> DbResult<()> {
        decrement_used_capacity(&mut **tx, &queue_id.into()).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(transparent)]
pub struct DbQueueId(Uuid);

impl DbQueueId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<Uuid> for DbQueueId {
    fn from(value: Uuid) -> Self {
        DbQueueId(value)
    }
}

impl From<DbQueueId> for QueueId {
    fn from(value: DbQueueId) -> Self {
        QueueId::new(value.0)
    }
}

impl From<&QueueId> for DbQueueId {
    fn from(value: &QueueId) -> Self {
        DbQueueId(value.inner())
    }
}

impl From<QueueId> for DbQueueId {
    fn from(value: QueueId) -> Self {
        DbQueueId(value.inner())
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Type)]
#[sqlx(type_name = "queue_status", rename_all = "UPPERCASE")]
pub enum DbQueueStatus {
    /// Job can be scheduled for execution
    #[default]
    Active,

    /// Job is not going to be scheduled for execution    
    Inactive,
}

impl From<DbQueueStatus> for QueueStatus {
    fn from(value: DbQueueStatus) -> Self {
        match value {
            DbQueueStatus::Active => QueueStatus::Active,
            DbQueueStatus::Inactive => QueueStatus::Inactive,
        }
    }
}

#[derive(Debug)]
pub struct QueueRow {
    /// Unique identifier for this queue
    queue_id: DbQueueId,

    /// The name of this queue
    queue_name: String,

    /// Status of the queue
    status: DbQueueStatus,

    /// Maximum number of jobs for this queue
    capacity: i32,

    /// Current number of jobs in this queue
    used_capacity: i32,

    /// Timestamp of queue creation
    created_at: DateTime<Utc>,

    /// Time when the job was last updated
    updated_at: DateTime<Utc>,
}

impl QueueRow {
    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }
}

impl From<QueueRow> for Queue {
    fn from(value: QueueRow) -> Self {
        Queue {
            queue_id: value.queue_id.into(),
            queue_name: value.queue_name,
            status: value.status.into(),
            capacity: value.capacity,
            used_capacity: value.used_capacity,
            created_at: value.created_at,
            updated_at: value.created_at,
        }
    }
}
