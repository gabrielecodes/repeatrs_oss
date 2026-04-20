// These bundles group repository factories together so that services can access
// multiple repositories in a single transaction. For example, `PgJobBundle`
// includes both `PgJobRepository` and `PgQueueRepository` since the `JobService`
// runs queries involving both the `jobs` and the `queues` tables.
//
// A bundle struct (e.g. `PgJobBundle`) is instantiated in the main and injected
// to the service that needs that bundle. The struct implements a trait (e.g. JobBundle)
// with factory methods that return the repositories with the queries that the service
// needs. The bundles themselves are just containers for factory methods. The Bundle
// struct does not need a transaction, it is injected by the database context provider.
// The bundle struct is instantiated as opposed to having only the bundle trait to avoid
// dynamic dispatch.
//
// Each service is given its own bundle via dependency injection (avoiding dynamic dispatch)
// at the service layer inside the transaction closure. The service uses the repositories
// in the  bundle to perform cross repository operations.

use async_nats::jetstream::Context;
use repeatrs_db::{PgJobRepository, PgQueueRepository};
use repeatrs_domain::{JobOperations, JobQueueOperations, QueueOperations};
use repeatrs_nats::NatsJobQueueRepository;

// -------- Job Service Bundle --------
pub trait JobBundle<E> {
    fn job_repo(&self) -> impl JobOperations<E> + Clone;
    fn queue_repo(&self) -> impl QueueOperations<E> + Clone;
}

#[derive(Default)]
pub struct PgJobBundle;

impl PgJobBundle {
    pub fn new() -> Self {
        Self
    }
}

impl<E> JobBundle<E> for PgJobBundle
where
    PgQueueRepository: QueueOperations<E> + Clone,
    PgJobRepository: JobOperations<E> + Clone,
{
    fn job_repo(&self) -> impl JobOperations<E> + Clone {
        PgJobRepository
    }

    fn queue_repo(&self) -> impl QueueOperations<E> + Clone {
        PgQueueRepository
    }
}

// -------- Queue Service Bundle --------
pub trait QueueBundle<E> {
    fn queue_repo(&self) -> impl QueueOperations<E> + Clone;
}

#[derive(Default)]
pub struct PgQueueBundle;

impl PgQueueBundle {
    pub fn new() -> Self {
        Self
    }
}

impl<E> QueueBundle<E> for PgQueueBundle
where
    PgQueueRepository: QueueOperations<E> + Clone,
{
    fn queue_repo(&self) -> impl QueueOperations<E> + Clone {
        PgQueueRepository
    }
}

// -------- Scheduler Service Bundle --------
pub trait JobQueueBundle<E> {
    fn job_repo(&self) -> impl JobOperations<E> + Clone;
    fn job_queue_repo(&self) -> impl JobQueueOperations + Clone;
}

pub struct NatsJobQueueBundle<'a> {
    context: Context,
    environment: &'a str,
    prefix: &'a str,
}

impl<'a> NatsJobQueueBundle<'a> {
    pub fn new(context: Context, environment: &'a str, prefix: &'a str) -> Self {
        Self {
            context,
            environment,
            prefix,
        }
    }
}

impl<'a, E> JobQueueBundle<E> for NatsJobQueueBundle<'a>
where
    PgJobRepository: JobOperations<E>,
{
    fn job_repo(&self) -> impl JobOperations<E> + Clone {
        PgJobRepository
    }

    fn job_queue_repo(&self) -> impl JobQueueOperations + Clone {
        NatsJobQueueRepository::new(self.context.clone(), self.environment, self.prefix)
    }
}
