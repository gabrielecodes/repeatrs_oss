use crate::IsId;

use std::{ops::Deref, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerId(Uuid);
impl IsId for WorkerId {}

impl Deref for WorkerId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for WorkerId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<WorkerId> for Uuid {
    fn from(job_id: WorkerId) -> Self {
        job_id.0
    }
}

impl FromStr for WorkerId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        Ok(WorkerId(Uuid::from_str(s)?))
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl WorkerId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn inner(self) -> Uuid {
        self.0
    }
}
