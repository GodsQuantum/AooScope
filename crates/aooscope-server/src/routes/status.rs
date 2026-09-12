use crate::{APP_VERSION, dto::StatusDto, error::ApiResult, state::AppState};
use aooscope_config::{load_settings, load_state};
use axum::{Json, extract::State};

#[utoipa::path(get, path = "/api/status", responses((status = 200, body = StatusDto)))]
pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<StatusDto>> {
    let settings = load_settings(&state.paths)?;
    let runtime = load_state(&state.paths).ok();
    let updated_unix = runtime
        .as_ref()
        .and_then(|doc| doc.meta.as_ref())
        .and_then(|meta| meta.get("updated_unix"))
        .and_then(|value| value.as_i64());
    Ok(Json(StatusDto {
        version: APP_VERSION.to_owned(),
        brightness: settings.display.brightness,
        native_brightness: false,
        device_present: state.device.exists(),
        updated_unix,
    }))
}
