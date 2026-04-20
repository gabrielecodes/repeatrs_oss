use super::{DbQueueId, DbQueueStatus, QueueRow};
use crate::DbResult;
use sqlx::{Executor, Postgres};

pub(crate) async fn add_queue<'e, E>(exec: E, name: &str) -> DbResult<DbQueueId>
where
    E: Executor<'e, Database = Postgres>,
{
    let queue_id: DbQueueId = sqlx::query_scalar!(
        r#"INSERT INTO queues (
                queue_name
            ) VALUES ($1)
            RETURNING queue_id as "queue_id: DbQueueId"
            "#,
        name
    )
    .fetch_one(exec)
    .await?;

    Ok(queue_id)
}

/// Returns all the queues.
pub(crate) async fn get_queues<'e, E>(exec: E) -> DbResult<Vec<QueueRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let queues: Vec<QueueRow> = sqlx::query_as!(
        QueueRow,
        r#"
        SELECT
            queue_id as "queue_id: DbQueueId",
            queue_name,
            status AS "status: DbQueueStatus",
            capacity,
            used_capacity,
            created_at,
            updated_at
        FROM queues
        "#
    )
    .fetch_all(exec)
    .await?;

    Ok(queues)
}

/// Returns the queues given their id.
pub(crate) async fn get_queues_by_id<'e, E>(
    exec: E,
    queue_ids: &[DbQueueId],
) -> DbResult<Vec<QueueRow>>
where
    E: Executor<'e, Database = Postgres>,
{
    let queues: Vec<QueueRow> = sqlx::query_as!(
        QueueRow,
        r#"
        SELECT
            queue_id as "queue_id: DbQueueId",
            queue_name,
            status AS "status: DbQueueStatus",
            capacity,
            used_capacity,
            created_at,
            updated_at
        FROM queues
        WHERE queue_id = ANY($1::uuid[])
        "#,
        &queue_ids as &[DbQueueId]
    )
    .fetch_all(exec)
    .await?;

    Ok(queues)
}

/// Returns a queue given its id.
pub(crate) async fn get_queue_by_id<'e, E>(exec: E, queue_id: &DbQueueId) -> DbResult<QueueRow>
where
    E: Executor<'e, Database = Postgres>,
{
    let queue_row: QueueRow = sqlx::query_as!(
        QueueRow,
        r#"
        SELECT
            queue_id as "queue_id: DbQueueId",
            queue_name,
            status AS "status: DbQueueStatus",
            capacity,
            used_capacity,
            created_at,
            updated_at
        FROM queues
        WHERE queue_id = $1
        "#,
        queue_id as &DbQueueId
    )
    .fetch_one(exec)
    .await?;

    Ok(queue_row)
}

/// Returns a queue id given its name if it exists.
pub(crate) async fn get_queue_by_name<'e, E>(exec: E, queue_name: &str) -> DbResult<QueueRow>
where
    E: Executor<'e, Database = Postgres>,
{
    let queue_id: QueueRow = sqlx::query_as!(
        QueueRow,
        r#"
            SELECT
                queue_id as "queue_id: DbQueueId",
                queue_name,
                status as "status: DbQueueStatus",
                capacity,
                used_capacity,
                created_at,
                updated_at
            FROM queues
            WHERE queue_name = $1
            "#,
        queue_name
    )
    .fetch_one(exec)
    .await?;

    Ok(queue_id)
}

// /// Deletes multiple jobs from the pool
// async fn delete_queue_by_id<'e, E>(exec: E, queue_id: &QueueId) -> DbResult<QueueId>
// where
//     E: Executor<'e, Database = Postgres>,
// {
//     let id = sqlx::query_scalar!(
//         r#"
//             DELETE FROM queues
//             WHERE queue_id = $1
//             RETURNING queue_id
//             "#,
//         queue_id
//     )
//     .fetch_one(exec)
//     .await?;

//     Ok(id)
// }

/// queries that require an instannce of a [`Queue`].
/// Increases the [`Queue::used_capacity`] of the queue given its id
pub(crate) async fn increment_used_capacity<'e, E>(exec: E, queue_id: &DbQueueId) -> DbResult<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query!(
        r#"
        UPDATE queues
        SET used_capacity = used_capacity + 1
        WHERE queue_id = $1 AND used_capacity < capacity
        RETURNING queue_id "queue_id: DbQueueId"
        "#,
        queue_id.0
    )
    .fetch_one(exec)
    .await?;
    Ok(())
}

/// Increases the [`Queue::used_capacity`] of the queue given its id
pub(crate) async fn decrement_used_capacity<'e, E>(exec: E, queue_id: &DbQueueId) -> DbResult<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query!(
        r#"
        UPDATE queues
        SET used_capacity = used_capacity - 1
        WHERE queue_id = $1 AND used_capacity > 0
        RETURNING queue_id "queue_id: DbQueueId"
        "#,
        queue_id.0
    )
    .fetch_one(exec)
    .await?;
    Ok(())
}
