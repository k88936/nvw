use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultOutcome {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamBound {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskPayload {
    pub param_count: usize,
    pub param_bounds: Vec<ParamBound>,
    pub max_iters: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTaskRequest {
    pub worker_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskLease {
    pub task_id: Uuid,
    pub lease_id: Uuid,
    pub worker_id: Uuid,
    pub leased_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
    pub attempt: i32,
    pub payload: TaskPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimTaskResponse {
    pub lease: Option<TaskLease>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRunMetrics {
    pub iters: usize,
    pub best_iters: usize,
    pub termination: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessfulOptimization {
    pub best_cost: f32,
    pub best_param: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailedOptimization {
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitTaskResultRequest {
    pub task_id: Uuid,
    pub lease_id: Uuid,
    pub worker_id: Uuid,
    pub outcome: ResultOutcome,
    pub metrics: TaskRunMetrics,
    pub success: Option<SuccessfulOptimization>,
    pub failure: Option<FailedOptimization>,
    pub finished_at: DateTime<Utc>,
}