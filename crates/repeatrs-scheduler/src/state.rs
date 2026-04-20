use sqlx::postgres::PgPool;

use crate::nats::JetstreamClient;

/// Application state
#[derive(Debug, Clone)]
pub struct State {
    /// Database connection pool
    pool: PgPool,

    /// Nats jetstream stream
    jetstream: JetstreamClient,
}

impl State {
    pub fn new(pool: PgPool, jetstream: JetstreamClient) -> Self {
        Self { pool, jetstream }
    }

    /// Returns a clone of the database connection pool.
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Returns a clone of the jetstream [`JetstreamClient`].
    pub fn jetstream(&self) -> JetstreamClient {
        self.jetstream.clone()
    }
}
