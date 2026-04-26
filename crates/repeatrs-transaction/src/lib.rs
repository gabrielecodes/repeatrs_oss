// use futures::future::BoxFuture;
use rand::RngExt;
use repeatrs_db::error::DbError;
use sqlx::{PgPool, Postgres, Transaction};
use std::error::Error;
use std::fmt::Debug;
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

// pub trait AsyncFnMut<'a, Arg> {
//     type Fut: Future<Output = Self::Output> + Send + 'a;
//     type Output;
//     fn call(&mut self, arg: &'a mut Arg) -> Self::Fut;
// }

// impl<'a, Arg: 'a, F, Fut, Out> AsyncFnMut<'a, Arg> for F
// where
//     F: FnMut(&'a mut Arg) -> Fut,
//     Fut: Future<Output = Out> + Send + 'a,
// {
//     type Fut = Fut;
//     type Output = Out;
//     fn call(&mut self, arg: &'a mut Arg) -> Self::Fut {
//         self(arg)
//     }
// }

pub trait AsyncTxFn<'a, 'tx, T, E>: Send {
    type Fut: Future<Output = Result<T, E>> + Send + 'a;
    fn call(&mut self, tx: &'a mut Transaction<'tx, Postgres>) -> Self::Fut;
}

impl<'a: 'tx, 'tx, T, Err, F, Fut> AsyncTxFn<'a, 'tx, T, Err> for F
where
    F: FnMut(&'a mut Transaction<'a, Postgres>) -> Fut + Send,
    Fut: Future<Output = Result<T, Err>> + Send + 'a,
{
    type Fut = Fut;
    fn call(&mut self, tx: &'a mut Transaction<'tx, Postgres>) -> Self::Fut {
        (self)(tx)
    }
}

#[async_trait::async_trait]
pub trait DatabaseContextProvider<'tx> {
    async fn execute<F, T: Send, Err>(&self, f: F) -> Result<T, Err>
    where
        F: for<'a> AsyncTxFn<'a, 'tx, T, Err> + Send + Sync,
        TransactionError: From<Err>,
        Err: Error + From<sqlx::Error> + Send + Sync + 'static;
}

#[async_trait::async_trait]
impl<'tx> DatabaseContextProvider<'tx> for DatabaseContext {
    async fn execute<F, T: Send, Err>(&self, mut f: F) -> Result<T, Err>
    where
        F: for<'a> AsyncTxFn<'a, 'tx, T, Err> + Send + Sync,
        TransactionError: From<Err>,
        Err: Error + From<sqlx::Error> + Send + Sync + 'static,
    {
        let mut attempts = 0;
        let max_attempts = 5;

        loop {
            let mut tx = self.pool.begin().await.unwrap();
            let trace_id = tracing::Span::current().id();

            if let Some(id) = trace_id {
                let tag_query = format!("SET LOCAL app.trace_id = '{}'", id.into_u64());
                sqlx::query(&tag_query).execute(&mut *tx).await.unwrap();
            }

            let boxed_closure = f.call(&mut tx);

            match boxed_closure.await {
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

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TransactionError {
    #[error("{0}")]
    Transaction(#[from] sqlx::Error),

    #[error("{0}")]
    Query(#[from] DbError),

    #[error("Transaction error at line {0}, file {1}")]
    Error(u32, &'static str),
}

// impl Error for TransactionError {}

// impl Display for TransactionError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             TransactionError::Transaction(e) => {
//                 write!(f, "{}", e)
//             }
//             TransactionError::Database(e) => {
//                 write!(f, "{}", e)
//             }
//             TransactionError::Error(line, file) => {
//                 write!(f, "line: {}, file: {}", line, file)
//             }
//         }
//     }
// }

// impl From<sqlx::Error> for TransactionError {
//     fn from(value: sqlx::Error) -> Self {
//         Self::Transaction(value)
//     }
// }

// impl From<DbError> for TransactionError {
//     fn from(value: DbError) -> Self {
//         Self::Database(value)
//     }
// }
