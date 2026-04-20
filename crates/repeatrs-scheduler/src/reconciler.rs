//! Scheduler-to-Worker reconciliation loop. 
//! 
//! The scheduler reconciler monitors workers and takes action if they are unresponsive.
//! 1. If there are recent heartbeats, the workers are assumed alive and running.
//! 2. If there aren't recent heartbeats: 
//!     - The reconciler tries to ping the machine:
//!         - If the ping is successful, the assumption is that the worker process/container exited.
//!             The worker process is respawned by systemd and restarts with a startup sequence
//!         - If the ping is unsuccessful the VM is set to UNREACHABLE and a termination sequence
//!             via cloud API is started. Upon successful termination the jobs are reinserted
//!             in the queue. 
//!         - Worker state transition: Running -> Heartbeat Timeout -> Ping Failure ->
//!             UNREACHABLE -> Termination API Call -> API Confirms Termination -> Re-enqueue Job.
//! 
//! TODO: 
//! Refinement: The scheduler's reconciler needs a secondary, much longer timeout. If a worker is pingable but has
//!     not resumed its heartbeat within a generous grace period (e.g., 30 minutes), it should be escalated. The
//!     reconciler should then override the "ping success" logic, declare the node STUCK, and initiate the same
//!     termination sequence used for unreachable VMs.

fn reconcile_worker_failure(failed_worker_id: WorkerId) {
    // Get all jobs managed by the failed worker
    let orphan_jobs = sqlx::query!("SELECT run_id FROM job_runs WHERE worker_id = $1", failed_worker_id);
    
    for job in orphan_jobs {
        // Query the External Metrics Sink (Vector's Destination)
        if metrics_store.is_active(job.id) {
            // CASE: Worker is dead, but VM/Container is alive.
            // ACTION: Spawn a new worker and trigger the Fencing Handshake.
            let new_worker = spawn_worker();
            // update job_runs with the new worker id
            trigger_handshake(new_worker, job.vm_ip, current_epoch + 1)
            // tell the VM to use the new epoch and worker id
        } else {
            // CASE: Both are dead.
            // ACTION: Mark job as FAILED (Node Lost).
            mark_job_failed(job.id, "WORKER_AND_VM_LOST")
        }
    }
}

fn reconcile_orphans() {
    // 1. Identify "Silent" VMs from the Heartbeat Table
    // A VM is silent if its managing Worker hasn't updated the unlogged table recently.
    let silent_vms = sqlx::query!(
        r#"
        SELECT DISTINCT ON (vm_id) 
            vm_id, 
            worker_id, 
            job_run_id, 
            epoch,
            created_at
        FROM vm_heartbeats
        WHERE worker_id = $1 AND epoch = $2
        ORDER BY vm_id, created_at DESC
        "#,
        self.worker_id,
        self.epoch
    ).fetch_all(&pool).await?;

    for vm in silent_vms {
        if Utc::now() - vm.created_at > 30s {
            // Check the "Second Opinion": Vector Metrics
            // This confirms if the Job Container is actually dead or just the Worker is unresponsive.
            let is_vector_active = vector_api.check_pulse(vm.job_run_id);
    
            if is_vector_active {
                // CASE: WORKER FAILURE
                // The job is running fine, but the Worker is gone.
                println!("Orphan detected: Job {vm.job_run_id} is alive, but Worker {vm.worker_id} is unresponsive.");
    
                // TRIGGER TAKEOVER
                let new_worker_id = worker_manager.spawn_worker();
                
                with db.transaction():
                    // Update the 'Topic' so the new worker starts polling this VM's status
                    db.execute("
                        UPDATE vm_heartbeats 
                        SET worker_id = %s, last_seen = NOW() 
                        WHERE vm_id = %s
                    ", (new_worker_id, vm.vm_id));
    
                    // Update the Job Run record
                    db.execute("UPDATE job_runs SET worker_id = %s WHERE id = %s", 
                               (new_worker_id, vm.job_run_id))
            }
            else{
                // CASE: VM OR NETWORK FAILURE
                // No heartbeat AND no metrics. The VM is likely toast.
                print("Critical Failure: VM {vm.vm_id} is unreachable. Marking job as failed.");
                
                with db.transaction():
                    db.execute("UPDATE job_runs SET status = 'FAILED' WHERE id = %s", (vm.job_run_id,));
                    db.execute("DELETE FROM vm_heartbeats WHERE vm_id = %s", (vm.vm_id,));
                    
                    // Free up the VM resources via Cloud API
                    cloud_provider.terminate_vm(vm.vm_id)
            }
        }
    }
}

// Run every 15 seconds
loop {
    reconcile_orphans();
    check_worker_status();
    sleep(15)
}

// run every 60 seconds
loop {
// -- Remove heartbeats for jobs that are no longer in an active state
// DELETE FROM vm_heartbeats 
// WHERE job_run_id IN (
//    SELECT id FROM job_runs 
//    WHERE status NOT IN ('PENDING', 'RUNNING')
}
/*
fn check_worker_status()

is worker is down:
    1. spawn new worker
    2. update worker_id in job_runs table
    UPDATE job_runs 
    SET 
        worker_id = $new_worker_id, 
        epoch = epoch + 1 
    WHERE id = $job_run_id 
    RETURNING epoch;

*/

/*
fn spawn_worker()
New Worker receives job_run_id and epoch from the Reconciler.
New Worker connects to the Job VM.
New Worker issues a "Re-attach" command: set_owner(worker_id, epoch).

The VM's logic:
- If new_epoch > current_epoch: Accept. Update internal state. Stop responding to the old worker_id.
- If new_epoch <= current_epoch: Reject. (This prevents an old, delayed message from a previous zombie worker from taking control).

*/