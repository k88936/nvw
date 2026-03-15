use chrono::{Duration as ChronoDuration, Utc};
use diesel::{Connection, SqliteConnection};
use proto::{
    ClaimTaskRequest, ClaimTaskResponse, ResultOutcome, SubmitTaskResultRequest, TaskLease,
    TaskPayload,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::repository::lease_repo::{
    deactivate_lease, insert_lease, latest_attempt_for_task, lease_by_ids,
};
use crate::repository::result_repo::{NewTaskResult, insert_task_result};
use crate::repository::task_repo::{fetch_first_pending_task, mark_task_running, mark_task_status};
use crate::app::AppState;

pub enum SubmitResult {
    Accepted,
    Conflict,
}

pub fn claim_task(state: &AppState, req: &ClaimTaskRequest) -> anyhow::Result<ClaimTaskResponse> {
    info!(worker_id = %req.worker_id, "claim request received");
    let mut conn = state.db_pool.get()?;
    let lease = claim_next_task(&mut conn, req, state.lease_timeout_secs)?;

    if let Some(ref leased) = lease {
        info!(
            worker_id = %leased.worker_id,
            task_id = %leased.task_id,
            lease_id = %leased.lease_id,
            attempt = leased.attempt,
            "task leased"
        );
    } else {
        info!(worker_id = %req.worker_id, "no task available to lease");
    }

    Ok(ClaimTaskResponse { lease })
}

fn claim_next_task(
    conn: &mut SqliteConnection,
    req: &ClaimTaskRequest,
    lease_timeout_secs: i64,
) -> Result<Option<TaskLease>, diesel::result::Error> {
    let now = Utc::now();
    let expires = now + ChronoDuration::seconds(lease_timeout_secs);
    let now_naive = now.naive_utc();
    let expires_naive = expires.naive_utc();

    conn.transaction::<Option<TaskLease>, diesel::result::Error, _>(|conn| {
        let pending_task = fetch_first_pending_task(conn)?;
        let Some(task) = pending_task else {
            return Ok(None);
        };

        let task_id_uuid = Uuid::parse_str(&task.task_id).map_err(|e| {
            diesel::result::Error::SerializationError(Box::new(std::io::Error::other(
                e.to_string(),
            )))
        })?;
        let payload: TaskPayload = serde_json::from_str(&task.payload_json).map_err(|e| {
            diesel::result::Error::SerializationError(Box::new(std::io::Error::other(
                e.to_string(),
            )))
        })?;

        let previous_attempt = latest_attempt_for_task(conn, &task.task_id)?;
        let attempt = previous_attempt.map(|v| v + 1).unwrap_or(1);

        let lease_id = Uuid::new_v4();
        let lease_id_s = lease_id.to_string();
        let worker_id_s = req.worker_id.to_string();
        insert_lease(
            conn,
            &lease_id_s,
            &task.task_id,
            &worker_id_s,
            now_naive,
            expires_naive,
            attempt,
        )?;
        mark_task_running(conn, &task.task_id, now_naive)?;

        Ok(Some(TaskLease {
            task_id: task_id_uuid,
            lease_id,
            worker_id: req.worker_id,
            leased_at: now,
            lease_expires_at: expires,
            attempt,
            payload,
        }))
    })
}

pub fn submit_task_result(
    state: &AppState,
    req: &SubmitTaskResultRequest,
) -> anyhow::Result<SubmitResult> {
    info!(
        worker_id = %req.worker_id,
        task_id = %req.task_id,
        lease_id = %req.lease_id,
        outcome = ?req.outcome,
        "submit request received"
    );

    let mut conn = state.db_pool.get()?;
    let outcome = persist_task_result(&mut conn, req)?;
    match outcome {
        SubmitResult::Accepted => {
            info!(
                worker_id = %req.worker_id,
                task_id = %req.task_id,
                lease_id = %req.lease_id,
                "result accepted"
            );
        }
        SubmitResult::Conflict => {
            warn!(
                worker_id = %req.worker_id,
                task_id = %req.task_id,
                lease_id = %req.lease_id,
                "result rejected due to stale or invalid lease"
            );
        }
    }
    Ok(outcome)
}

fn persist_task_result(
    conn: &mut SqliteConnection,
    req: &SubmitTaskResultRequest,
) -> Result<SubmitResult, diesel::result::Error> {
    let now = Utc::now().naive_utc();
    let lease_id_s = req.lease_id.to_string();
    let task_id_s = req.task_id.to_string();
    let worker_id_s = req.worker_id.to_string();
    let outcome_s = match req.outcome {
        ResultOutcome::Succeeded => "succeeded",
        ResultOutcome::Failed => "failed",
    };
    let best_param_json = req
        .success
        .as_ref()
        .map(|s| serde_json::to_string(&s.best_param))
        .transpose()
        .map_err(|e| diesel::result::Error::SerializationError(Box::new(e)))?;

    let result = conn.transaction::<(), diesel::result::Error, _>(|conn| {
        let lease = lease_by_ids(conn, &lease_id_s, &task_id_s, &worker_id_s)?;
        let Some((active, expires_at)) = lease else {
            return Err(diesel::result::Error::NotFound);
        };
        if !active || expires_at < Utc::now().naive_utc() {
            return Err(diesel::result::Error::RollbackTransaction);
        }

        insert_task_result(
            conn,
            NewTaskResult {
                lease_id: &lease_id_s,
                task_id: &task_id_s,
                worker_id: &worker_id_s,
                outcome: outcome_s,
                best_cost: req.success.as_ref().map(|s| s.best_cost),
                best_param_json: best_param_json.as_deref(),
                iters: req.metrics.iters as i32,
                best_iters: req.metrics.best_iters as i32,
                termination: &req.metrics.termination,
                error_message: req.failure.as_ref().map(|f| f.error_message.as_str()),
                finished_at: req.finished_at.naive_utc(),
            },
        )?;
        deactivate_lease(conn, &lease_id_s)?;
        mark_task_status(conn, &task_id_s, outcome_s, now)?;
        Ok(())
    });

    match result {
        Ok(()) => Ok(SubmitResult::Accepted),
        Err(diesel::result::Error::NotFound) | Err(diesel::result::Error::RollbackTransaction) => {
            Ok(SubmitResult::Conflict)
        }
        Err(e) => Err(e),
    }
}
