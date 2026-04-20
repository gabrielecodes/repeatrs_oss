use crate::{QueueId, Worker, WorkerId, WorkerStatus, error::Result};
use sqlx::{Executor, Postgres};

impl Worker {
    pub async fn register_worker<'e, E>(
        exec: E,
        queue_id: QueueId,
        vpc_id: &str,
        dns: &str,
        private_ip: &str,
    ) -> Result<()>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query!(
            r#"
            INSERT INTO workers (queue_id,  vpc_id, dns, private_ip)
            VALUES ($1, $2, $3, $4)
            "#,
            queue_id as QueueId,
            vpc_id,
            dns,
            private_ip
        )
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn get_worker<'e, E>(exec: E, worker_id: &WorkerId) -> Result<Worker>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query_as!(
            Worker,
            r#"SELECT 
                worker_id as "worker_id: WorkerId",
                queue_id as "queue_id: QueueId",
                status as "status: WorkerStatus",
                epoch,
                vpc_id,
                dns,
                private_ip,
                created_at
            FROM workers 
            WHERE worker_id = $1"#,
            worker_id.0
        )
        .fetch_one(exec)
        .await?;

        Ok(row)
    }

    pub async fn get_status<'e, E>(&self, exec: E) -> Result<WorkerStatus>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query!(
            r#"SELECT status as "status: WorkerStatus" FROM workers WHERE worker_id = $1"#,
            self.worker_id as WorkerId
        )
        .fetch_one(exec)
        .await?;

        Ok(row.status)
    }

    pub async fn get_workers_by_status<'e, E>(
        exec: E,
        status: WorkerStatus,
    ) -> Result<Vec<WorkerId>>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let ids = sqlx::query_scalar!(
            r#"SELECT worker_id as "worker_id: WorkerId" FROM workers WHERE status = $1"#,
            status as WorkerStatus
        )
        .fetch_all(exec)
        .await?;

        Ok(ids)
    }
}
