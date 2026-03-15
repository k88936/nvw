use std::sync::Arc;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use proto::{
    ClaimTaskRequest, ClaimTaskResponse, CreateTaskRequest, ListTasksResponse,
    SubmitTaskResultRequest, TaskDto, UpdateTaskRequest,
};
use crate::app::{conflict_error, internal_error, not_found_error, ApiResult, AppState};
use crate::service;
use uuid::Uuid;

pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> ApiResult<Json<TaskDto>> {
    let task = service::create_task(&state, &req).map_err(internal_error)?;
    Ok(Json(task))
}

pub async fn list_tasks(State(state): State<Arc<AppState>>) -> ApiResult<Json<ListTasksResponse>> {
    let tasks = service::list_tasks(&state).map_err(internal_error)?;
    Ok(Json(ListTasksResponse { tasks }))
}

pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResult<Json<TaskDto>> {
    let task = service::get_task(&state, task_id).map_err(internal_error)?;
    match task {
        Some(t) => Ok(Json(t)),
        None => Err(not_found_error("task not found")),
    }
}

pub async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> ApiResult<Json<TaskDto>> {
    let task = service::update_task(&state, task_id, &req).map_err(internal_error)?;
    match task {
        Some(t) => Ok(Json(t)),
        None => Err(not_found_error("task not found")),
    }
}

pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let deleted = service::delete_task(&state, task_id).map_err(internal_error)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found_error("task not found"))
    }
}

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