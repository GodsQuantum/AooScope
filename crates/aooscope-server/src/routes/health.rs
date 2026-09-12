use crate::{APP_VERSION, dto::HealthDto};
use axum::Json;

#[utoipa::path(get, path = "/api/health", responses((status = 200, body = HealthDto)))]
pub async fn get_health() -> Json<HealthDto> {
    Json(HealthDto {
        ok: true,
        version: APP_VERSION.to_owned(),
    })
}
