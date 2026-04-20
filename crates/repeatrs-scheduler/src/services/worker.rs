#![allow(unused)]
use repeatrs_proto::repeatrs::AddJobRequest;

use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{error, info};

#[derive(Clone)]
pub struct WorkerService {
    pub pool: PgPool,
}
