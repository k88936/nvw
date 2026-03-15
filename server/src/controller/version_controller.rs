use crate::app::{ApiResult, SharedState};
use axum::Json;
use axum::extract::State;
use proto::Version;

pub async fn handle_version(State(_state): State<SharedState>) -> ApiResult<Json<Version>> {
    Ok(Json(Version::default()))
}
