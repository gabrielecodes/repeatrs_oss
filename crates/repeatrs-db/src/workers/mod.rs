// mod failover;
// pub mod queries;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

use repeatrs_domain::{QueueId, WorkerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct DbWorkerId(pub Uuid);

impl From<Uuid> for DbWorkerId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<DbWorkerId> for WorkerId {
    fn from(value: DbWorkerId) -> Self {
        WorkerId::new(value.0)
    }
}

impl From<&WorkerId> for DbWorkerId {
    fn from(value: &WorkerId) -> Self {
        DbWorkerId(value.inner())
    }
}

impl From<WorkerId> for DbWorkerId {
    fn from(value: WorkerId) -> Self {
        DbWorkerId(value.inner())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, sqlx::Type)]
#[sqlx(type_name = "worker_status", rename_all = "UPPERCASE")]
pub enum WorkerStatus {
    Starting,
    Alive,
    Terminated,
    Terminating,
    Stale,
}

impl Display for WorkerStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            WorkerStatus::Starting => "STARTING",
            WorkerStatus::Alive => "ALIVE",
            WorkerStatus::Terminated => "TERMINATED",
            WorkerStatus::Terminating => "TERMINATING",
            WorkerStatus::Stale => "STALE",
        };
        f.write_str(s)
    }
}

impl FromStr for WorkerStatus {
    type Err = String;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "STARTING" => Ok(WorkerStatus::Starting),
            "ALIVE" => Ok(WorkerStatus::Alive),
            "TERMINATED" => Ok(WorkerStatus::Terminated),
            "TERMINATING" => Ok(WorkerStatus::Terminating),
            "STALE" => Ok(WorkerStatus::Stale),
            _ => Err("VM status not understood".into()),
        }
    }
}

pub struct WorkerRow {
    worker_id: DbWorkerId,
    queue_id: QueueId,
    status: WorkerStatus,
    epoch: i32,
    vpc_id: String,
    dns: String,
    private_ip: String,
    created_at: DateTime<Utc>,
}
