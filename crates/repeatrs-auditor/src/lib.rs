//! The auditor acts as a truth reconciliation mechanism checking and recording the status
//! of the worker VMs and job runs in the database. It is responsible for:
//!
//! 1. monitoring the workers' health and esuring that they are up and running.
//! 2. Updating the job run status aligning the database with the message queue.
//!
//! ## 1. Worker lifecycle
//!
//! Workers' health is monitored by querying an unlogged table of heartbeats and eventually
//! respawning unresponsive workers. Is a worker is unresponsive for more than a certain amount
//! of time, the auditor checks the status of the VM via the cloud provider API and if the VM is down,
//! a VM termination procedure is started and a new VM is provisioned. The jobs that were running
//! on that machine are considered failed and are requeued.
//!
//! If the VM is up but the worker is unresponsive, the auditor marks the worker as unresponsive
//! tries to respawn it and to reattach the containers to the worker. If that fails, the jobs are
//! considered failed and the machine is terminated and reprovisioned.
//!
//! ## 2. Job status reconciliation
//!
//! Workers start jobs updating the queue job status queue at every status change. The auditor
//! consumes these messages at regular intevals or when a maximum number of messages is reached,
//! and updates the database accordingly.
//!

pub mod api;
pub mod error;

use sqlx::PgPool;

pub struct State {
    pool: PgPool,
}
