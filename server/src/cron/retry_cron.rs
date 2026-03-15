use crate::app::{AppState, SharedState};
use crate::repository::task_repo;
use tracing::error;

pub fn spawn_retry_cron(state: SharedState) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = retry_failed(&state) {
                error!("retry failed: {}", e);
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(64)).await;
        }
    });
}

pub fn retry_failed(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state.db_pool.get()?;
    task_repo::requeue_failed_task(&mut conn)?;
    Ok(())
}
