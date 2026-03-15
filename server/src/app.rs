use std::sync::Arc;

use axum::{Json, http::StatusCode};
use diesel::{
    SqliteConnection,
    r2d2::{ConnectionManager, Pool},
};
use serde::{Deserialize, Serialize};
use tracing::error;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub lease_timeout_secs: i64,
    pub bearer_token: String,
}

pub type SharedState = Arc<AppState>;
pub type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}

pub fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<ErrorResponse>) {
    error!(error = %err, "internal server error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            message: err.to_string(),
        }),
    )
}

pub fn conflict_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            message: message.to_string(),
        }),
    )
}
