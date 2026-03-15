use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
pub struct TaskPayload {
    pub swarm_scale: usize,
    pub param_bounds_min: Vec<f64>,
    pub param_bounds_max: Vec<f64>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub payload: TaskPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<TaskStatus>,
    pub payload: Option<TaskPayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDto {
    pub id: Uuid,
    pub status: TaskStatus,
    pub payload: TaskPayload,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResultDto {
    pub task_id: Uuid,
    pub lease_id: Uuid,
    pub worker_id: Uuid,
    pub outcome: ResultOutcome,
    pub metrics: TaskRunMetrics,
    pub success: Option<SuccessfulOptimization>,
    pub failure: Option<FailedOptimization>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
impl Default for Version {
    fn default() -> Self {
        Self{
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}
