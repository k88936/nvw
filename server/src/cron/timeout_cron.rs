use std::time::Duration;
use chrono::Utc;
use tracing::{error, info, warn};

use crate::{app::AppState};
use crate::app::SharedState;
use crate::repository::lease_repo::deactivate_expired_leases;

pub fn mark_expired_as_failed(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state.db_pool.get()?;
    let expired_count =
        deactivate_expired_leases(&mut conn, Utc::now().naive_utc())?;
    if expired_count > 0 {
        warn!(
            expired_count,
            "expiring stale leases and failing running tasks"
        );
    }
    Ok(())
}

pub fn spawn_timeout_job(state: SharedState) {
    info!("timeout monitor started");
    tokio::spawn(async move {
        loop {
            if let Err(e) = mark_expired_as_failed(&state) {
                error!("timeout job error: {}", e);
            }
            tokio::time::sleep(Duration::from_secs(64)).await;
        }
    });
}