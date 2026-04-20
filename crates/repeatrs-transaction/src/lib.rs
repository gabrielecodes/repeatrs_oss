use futures::future::BoxFuture;
use rand::RngExt;
use repeatrs_db::error::DbError;
use sqlx::{PgPool, Postgres, Transaction};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::result::Result;
use tokio::time::Duration;

#[derive(Clone)]
pub struct DatabaseContext {
    pool: PgPool,
}

impl DatabaseContext {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
pub trait DatabaseContextProvider<'tx, E> {
    async fn execute<F, T, Err>(&self, f: F) -> Result<T, Err>
    where
        F: for<'a> FnMut(&'a mut E) -> BoxFuture<'a, Result<T, Err>> + Send,
        T: Send,
        Err: Error + From<sqlx::Error> + Send;
}

#[async_trait::async_trait]
impl<'tx> DatabaseContextProvider<'tx, Transaction<'tx, Postgres>> for DatabaseContext {
    async fn execute<F, T, Err>(&self, mut f: F) -> Result<T, Err>
    where
        F: for<'a> FnMut(&'a mut Transaction<'tx, Postgres>) -> BoxFuture<'a, Result<T, Err>>
            + Send,
        T: Send,
        Err: Error + From<sqlx::Error> + Send,
    {
        let mut attempts = 0;
        let max_attempts = 5;

        loop {
            let mut tx = self.pool.begin().await?;
            let trace_id = tracing::Span::current().id();

            if let Some(id) = trace_id {
                let tag_query = format!("SET LOCAL app.trace_id = '{}'", id.into_u64());
                sqlx::query(&tag_query).execute(&mut *tx).await?;
            }

            match f(&mut tx).await {
                Ok(result) => match tx.commit().await {
                    Ok(_) => return Ok(result),
                    Err(e) => {
                        if !matches!(get_retry_category(&e), RetryPolicy::Fail)
                            && attempts < max_attempts
                        {
                            attempts += 1;
                            backoff_with_jitter(attempts).await;
                            continue;
                        }
                        return Err(Err::from(e));
                    }
                },
                Err(e) => {
                    if attempts < max_attempts {
                        attempts += 1;
                        backoff_with_jitter(attempts).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

async fn backoff_with_jitter(attempt: u32) {
    // 2^attempt * 10ms
    let base_ms = 2_u64.pow(attempt) * 10;

    let jitter = rand::rng().random_range(0..20);

    tokio::time::sleep(Duration::from_millis(base_ms + jitter)).await;
}

enum RetryPolicy {
    Immediate,
    Backoff,
    Fail,
}

// Helper to identify Postgres transient errors
fn get_retry_category(err: &sqlx::Error) -> RetryPolicy {
    if let sqlx::Error::Database(db_err) = err {
        let policy = match db_err.code().as_deref() {
            Some("40001") | Some("400P1") | Some("55P03") => RetryPolicy::Immediate,
            Some("57P01") | Some("57P03") | Some("08006") => RetryPolicy::Backoff,
            _ => RetryPolicy::Fail,
        };

        return policy;
    }

    RetryPolicy::Fail
}

#[derive(Debug)]
#[non_exhaustive]
pub enum TransactionError {
    Transaction(sqlx::Error),
    Database(DbError),
    Error(u32, &'static str),
}

impl Error for TransactionError {}

impl Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionError::Transaction(e) => {
                write!(f, "{}", e)
            }
            TransactionError::Database(e) => {
                write!(f, "{}", e)
            }
            TransactionError::Error(line, file) => {
                write!(f, "line: {}, file: {}", line, file)
            }
        }
    }
}

impl From<sqlx::Error> for TransactionError {
    fn from(value: sqlx::Error) -> Self {
        Self::Transaction(value)
    }
}

impl From<DbError> for TransactionError {
    fn from(value: DbError) -> Self {
        Self::Database(value)
    }
}
