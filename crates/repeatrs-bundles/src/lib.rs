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

use repeatrs_db::{
    PgJobRepository, PgJobRunRepository, PgJobScheduleStateRepository, PgQueueRepository,
};
use repeatrs_domain::{
    JobOperations, JobRunOperations, JobScheduleStateOperations, QueueOperations,
};

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
pub trait JobSchedulerBundle<E> {
    fn job_repo(&self) -> impl JobOperations<E> + Clone;
    fn job_runs_repo(&self) -> impl JobRunOperations<E> + Clone;
    fn job_schedule_state_repo(&self) -> impl JobScheduleStateOperations<E> + Clone;
}

pub struct PgJobSchedulerBundle;

impl PgJobSchedulerBundle {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PgJobSchedulerBundle {
    fn default() -> Self {
        Self
    }
}

impl<E> JobSchedulerBundle<E> for PgJobSchedulerBundle
where
    PgJobRepository: JobOperations<E> + Clone,
    PgJobRunRepository: JobRunOperations<E> + Clone,
    PgJobScheduleStateRepository: JobScheduleStateOperations<E> + Clone,
{
    fn job_repo(&self) -> impl JobOperations<E> + Clone {
        PgJobRepository
    }

    fn job_runs_repo(&self) -> impl JobRunOperations<E> + Clone {
        PgJobRunRepository
    }
    fn job_schedule_state_repo(&self) -> impl JobScheduleStateOperations<E> + Clone {
        PgJobScheduleStateRepository
    }
}
