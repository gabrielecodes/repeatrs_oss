use tokio::time::Duration;

use crate::State;

pub struct JobStatusController {
    state: State,
}

impl JobStatusController {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub async fn start_job_status_reconciliation(state: State) {
        let duration = Duration::from_secs(10);
        // let health_controller = HealthController::new(self.state.pool.clone());

        loop {
            // consume the messages from the queue and update the database accordingly
            tokio::time::sleep(duration).await;
        }
    }
}
