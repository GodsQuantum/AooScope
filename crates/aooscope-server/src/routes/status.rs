use crate::{dto::StatusDto, error::ApiResult, state::AppState};
use axum::{Json, extract::State};

#[utoipa::path(get, path = "/api/status", responses((status = 200, body = StatusDto)))]
pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<StatusDto>> {
    Ok(Json(state.refresh_status()))
}
