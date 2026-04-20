use crate::{Job, JobQueue, Result};

use async_nats::jetstream;

impl JobQueue {
    /// Insert a new job in the job queue given the queue name and the job itself
    #[tracing::instrument(skip_all)]
    pub async fn publish_job(js: &jetstream::Context, jobs: &[Job]) -> Result<()> {
        for job in jobs.iter() {
            let subject = format!("job_queues.{}.{}", job.queue_id(), job.job_id());
            js.publish(subject, "".into()).await?.await?;
        }
        Ok(())
    }
}

// impl JobQueue {
//     /// Insert a new job in the job queue given the queue name and the job itself
//     #[tracing::instrument(skip(js), fields(queue_id = %queue_id, job_id = %job_id))]
//     async fn publish_job(
//         js: &jetstream::Context,
//         queue_id: &QueueId,
//         job_id: &JobId,
//     ) -> Result<()> {
//         let subject = format!("job_queue.{queue_name}.{job_name}");
//         js.publish(subject, payload.into()).await?.await?;
//         Ok(())
//     }

//     pub async fn add_job<'e, E>(exec: E, queue_id: &QueueId, job_id: &JobId) -> Result<JobQueueId>
//     where
//         E: Executor<'e, Database = Postgres>,
//     {
//         let queue_id = sqlx::query_as!(
//             JobQueueId,
//             r#"INSERT INTO job_queues (
//                 queue_id,
//                 job_id
//             )
//             VALUES ($1, $2)
//             RETURNING job_queue_id"#,
//             queue_id,
//             job_id
//         )
//         .fetch_one(&exec)
//         .await
//         .map_query_err()?;

//         Ok(queue_id)
//     }

//     pub async fn insert_many() {}

//     #[tracing::instrument(skip_all)]
//     pub async fn get_default_queue<'e, E>(exec: E) -> Result<JobQueue>
//     where
//         E: Executor<'e, Database = Postgres>,
//     {
//         let pool = sqlx::query_as!(
//             JobQueue,
//             r#"SELECT * FROM job_queues WHERE name = 'default'"#,
//         )
//         .fetch_one(&exec)
//         .await?;

//         Ok(pool)
//     }

//     /// Returns the job queue id given its name
//     #[tracing::instrument(skip(exec), fields(name = %name))]
//     pub async fn get_queue_id_by_name<'e, E>(exec: E, name: &str) -> Result<JobQueueId>
//     where
//         E: Executor<'e, Database = Postgres>,
//     {
//         let queue_id = sqlx::query_as!(
//             JobQueueId,
//             r#"SELECT job_queue_id FROM pools WHERE name = $1"#,
//             name
//         )
//         .fetch_optional(&exec)
//         .await?;

//         Ok(queue_id)
//     }

//     /// Inserts multiple jobs in the queue with updated deadlines.
//     pub async fn reinsert<'e, E>(exec: E, job_ids: impl IntoIterator<Item = &JobId>) -> Result<()>
//     where
//         E: Acquire<'e, Database = Postgres>,
//     {
//         let ids: Vec<Uuid> = job_ids.into_iter().map(|j| j.0).collect();

//         if ids.is_empty() {
//             return Ok(());
//         }

//         let mut tx = exec.begin().await?;

//         let jobs_info = sqlx::query!(
//             "
//             SELECT
//                 job_id,
//                 schedule,
//                 pool_id
//             FROM jobs
//             WHERE job_id = ANY($1::uuid[])
//             ",
//             &ids
//         )
//         .fetch_all(&mut *tx)
//         .await?;

//         let data: Vec<_> = jobs_info
//             .into_iter()
//             .map(|job| {
//                 let deadline: Option<DateTime<Utc>> =
//                     if let Ok(cron_schedule) = Cron::from_str(&job.schedule) {
//                         cron_schedule.find_next_occurrence(&Utc::now(), false).ok()
//                     } else {
//                         None
//                     };
//                 (job.job_id, deadline, job.pool_id)
//             })
//             .collect();

//         if !data.is_empty() {
//             let mut query = QueryBuilder::<Postgres>::new(
//                 "INSERT INTO job_queues (job_id, deadline, pool_id) ",
//             );

//             query.push_values(data, |mut b, item| {
//                 b.push_bind(item.0).push_bind(item.1).push_bind(item.2);
//             });

//             query.push(" RETURNING job_id::uuid");

//             let _ = query.build().fetch_all(&mut *tx).await.map_query_err()?;
//         }

//         let _ = tx.commit().await?;

//         // TODO: handle jobs that have not been inserted

//         Ok(())
//     }
// }
