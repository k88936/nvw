use std::sync::Arc;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use proto::{ClaimTaskRequest, ClaimTaskResponse, SubmitTaskResultRequest};
use crate::app::{conflict_error, internal_error, ApiResult, AppState};
use crate::service;

pub async fn claim_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimTaskRequest>,
) -> ApiResult<Json<ClaimTaskResponse>> {
    let response = service::claim_task(&state, &req).map_err(internal_error)?;
    Ok(Json(response))
}

pub async fn submit_task_result(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitTaskResultRequest>,
) -> ApiResult<StatusCode> {
    let outcome = service::submit_task_result(&state, &req).map_err(internal_error)?;
    match outcome {
        service::SubmitResult::Accepted => Ok(StatusCode::NO_CONTENT),
        service::SubmitResult::Conflict => Err(conflict_error("invalid, stale, or expired lease")),
    }
}