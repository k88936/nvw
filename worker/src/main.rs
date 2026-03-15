mod optimize;
pub mod problem;
pub mod utils;

use std::{env, time::Duration};

use anyhow::Result;
use chrono::Utc;
use optimize::{SolverResult, run_optimization};
use tracing::{error, info, warn};
use uuid::Uuid;
use proto::{ClaimTaskRequest, ClaimTaskResponse, SubmitTaskResultRequest};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let server_url =
        env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let worker_id = env::var("WORKER_ID")
        .ok()
        .and_then(|v| Uuid::parse_str(&v).ok())
        .unwrap_or_else(Uuid::new_v4);
    let poll_interval_ms: u64 = env::var("POLL_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4096);

    let client = reqwest::Client::new();
    info!(
        worker_id = %worker_id,
        server_url = %server_url,
        poll_interval_ms,
        "worker started"
    );

    loop {
        info!(worker_id = %worker_id, "polling for task");
        let claim_resp = client
            .post(format!("{}/v1/tasks/claim", server_url))
            .json(&ClaimTaskRequest { worker_id })
            .send()
            .await?;

        if !claim_resp.status().is_success() {
            error!("claim request failed: {}", claim_resp.status());
            tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            continue;
        }

        let claimed: ClaimTaskResponse = claim_resp.json().await?;
        let Some(lease) = claimed.lease else {
            info!(worker_id = %worker_id, "no task available");
            tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            continue;
        };
        info!(
            worker_id = %worker_id,
            task_id = %lease.task_id,
            lease_id = %lease.lease_id,
            attempt = lease.attempt,
            "task claimed"
        );

        let (outcome, success, failure, metrics) = match run_optimization(&lease.payload) {
            SolverResult::Success(ok, metrics) => (proto::ResultOutcome::Succeeded, Some(ok), None, metrics),
            SolverResult::Failure(err, metrics) => (proto::ResultOutcome::Failed, None, Some(err), metrics),
        };
        match outcome {
            proto::ResultOutcome::Succeeded => info!(
                worker_id = %worker_id,
                task_id = %lease.task_id,
                iters = metrics.iters,
                "optimization succeeded"
            ),
            proto::ResultOutcome::Failed => warn!(
                worker_id = %worker_id,
                task_id = %lease.task_id,
                iters = metrics.iters,
                "optimization failed"
            ),
        }

        let submit_body = SubmitTaskResultRequest {
            task_id: lease.task_id,
            lease_id: lease.lease_id,
            worker_id,
            outcome,
            metrics,
            success,
            failure,
            finished_at: Utc::now(),
        };

        let submit_resp = client
            .post(format!("{}/v1/tasks/result", server_url))
            .json(&submit_body)
            .send()
            .await?;
        if !submit_resp.status().is_success() {
            error!("submit request failed: {}", submit_resp.status());
        } else {
            info!(
                worker_id = %worker_id,
                task_id = %lease.task_id,
                lease_id = %lease.lease_id,
                status = %submit_resp.status(),
                "result submitted"
            );
        }
    }
}
