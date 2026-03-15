use crate::app::SharedState;
use crate::repository::task_repo;

pub fn spawn_retry_cron(state: SharedState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(64)).await;
        }
    });
}

pub fn retry_failed(state: SharedState) -> anyhow::Result<()> {
    let mut conn = state.db_pool.get()?;
    task_repo::requeue_failed_task(&mut conn)?;
    Ok(())
}