//! Implementation of business logic/invariants for the [`Queue`] entity.
//!
//! The service layer is responsible for:
//!
//! - Implementing business logic and invariants related to [`Queue`] management. Rules
//!   that involves multiple entities are enforced. These include:
//!     - Uniqueness or existence checks,
//!     - Aggregate rules (e.g. respecting limits/quotas involving multiple entities).
//!     - Workflow rules (e.g. forbidden state changes or entity creation).
//! - Interacting with the repository to perform database operations.
//! - Providing a clear API for controllers.

use crate::{ApiResult, err_ctx, error::ToApiError};
use repeatrs_bundles::QueueBundle;
use repeatrs_domain::{Queue, QueueId, QueueOperations};
use repeatrs_proto::repeatrs::QueueItem;
use repeatrs_transaction::{DatabaseContextProvider, TransactionError};
use std::marker::PhantomData;
use tracing::instrument;

pub struct QueueService<E, Q, D>
where
    Q: QueueBundle<E>,
    D: for<'tx> DatabaseContextProvider<'tx, E>,
{
    bundle: Q,
    context: D,
    _marker: PhantomData<E>,
}

impl<E, Q, D> QueueService<E, Q, D>
where
    Q: QueueBundle<E> + Send + Sync + 'static,
    D: for<'tx> DatabaseContextProvider<'tx, E> + Send + Sync + 'static,
    E: Sync + Send + 'static,
{
    pub fn new(bundle: Q, context: D) -> Self {
        Self {
            bundle,
            context,
            _marker: Default::default(),
        }
    }

    #[instrument(skip_all, err)]
    pub async fn get_queue_names(&self) -> ApiResult<Vec<String>> {
        let queue_repo = self.bundle.queue_repo();

        let queues = self
            .context
            .execute(|tx| {
                let queue_repo = queue_repo.clone();

                Box::pin(async move {
                    let queues = err_ctx!(queue_repo.get_queues(tx).await)?;

                    Ok::<Vec<Queue>, TransactionError>(queues)
                })
            })
            .await
            .map_transaction_error(line!(), file!())?;

        let names: Vec<String> = queues.iter().map(|q| q.queue_name.to_string()).collect();

        Ok(names)
    }

    /// Returns the queue corresponding to the given [`QueueId`].
    #[instrument(skip_all, err)]
    pub async fn get_queue_by_id(&self, queue_id: QueueId) -> ApiResult<Queue> {
        let queue_repo = self.bundle.queue_repo();

        let queue = self
            .context
            .execute(|tx| {
                let queue_repo = queue_repo.clone();

                Box::pin(async move {
                    let queue = err_ctx!(queue_repo.get_queue_by_id(tx, &queue_id).await)?;

                    Ok::<Queue, TransactionError>(queue)
                })
            })
            .await;

        Ok(err_ctx!(queue)?)
    }

    /// Returns all the queues
    #[instrument(skip_all, err)]
    pub async fn get_queues(&self) -> ApiResult<Vec<QueueItem>> {
        let queue_repo = self.bundle.queue_repo();

        let queues = self
            .context
            .execute(move |tx| {
                let queue_repo = queue_repo.clone();

                Box::pin(async move {
                    let queues = err_ctx!(queue_repo.get_queues(tx).await)?;

                    let queue_items: Vec<QueueItem> =
                        queues.iter().map(|q| q.to_queue_item()).collect();

                    Ok::<Vec<QueueItem>, TransactionError>(queue_items)
                })
            })
            .await;

        Ok(err_ctx!(queues)?)
    }
}

// fn to_subjects(strings: Vec<String>) -> core::result::Result<Vec<Subject>, std::str::Utf8Error> {
//     strings.into_iter().map(Subject::from_utf8).collect()
// }
