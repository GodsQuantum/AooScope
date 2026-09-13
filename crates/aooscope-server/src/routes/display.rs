use crate::{
    dto::{DisplayCapabilitiesDto, DisplayPowerDto, DisplayPowerRequest, StatusDto},
    error::{ApiError, ApiResult},
    state::AppState,
};
use aooscope_config::atomic_write_json;
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::Value;
use std::fs;

#[utoipa::path(get, path = "/api/display/capabilities", responses((status = 200, body = DisplayCapabilitiesDto)))]
pub async fn get_capabilities(State(state): State<AppState>) -> Json<DisplayCapabilitiesDto> {
    Json(DisplayCapabilitiesDto {
        power_on: state.display.power_state(),
        ..state.display_capabilities.into()
    })
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DisplayLuminanceRequest {
    pub value: i16,
}

#[utoipa::path(post, path = "/api/display/luminance", request_body = DisplayLuminanceRequest, responses((status = 200, body = StatusDto)))]
pub async fn set_luminance(
    State(state): State<AppState>,
    Json(request): Json<DisplayLuminanceRequest>,
) -> ApiResult<Json<StatusDto>> {
    if !(0..=100).contains(&request.value) {
        return Err(ApiError::BadRequest("luminance must be between 0 and 100"));
    }
    let _guard = state.settings_lock.lock().expect("settings lock poisoned");
    let path = state.paths.settings();
    let bytes = fs::read(&path).map_err(|source| aooscope_config::ConfigError::Read {
        path: path.clone(),
        source,
    })?;
    let mut document: Value =
        serde_json::from_slice(&bytes).map_err(|source| aooscope_config::ConfigError::Json {
            path: path.clone(),
            source,
        })?;
    let display = document
        .get_mut("display")
        .and_then(Value::as_object_mut)
        .ok_or(ApiError::BadRequest("settings display unavailable"))?;
    display.insert("brightness".into(), Value::from(request.value as u8));
    atomic_write_json(&path, &document)?;
    Ok(Json(state.refresh_status()))
}

#[utoipa::path(post, path = "/api/display/power", request_body = DisplayPowerRequest, responses((status = 200, body = DisplayPowerDto)))]
pub async fn set_power(
    State(state): State<AppState>,
    Json(request): Json<DisplayPowerRequest>,
) -> ApiResult<Json<DisplayPowerDto>> {
    if !state.display_capabilities.power_control {
        return Err(ApiError::BadRequest("display power control unavailable"));
    }
    if request.on {
        state.display.power_on().await?;
    } else {
        state.display.power_off().await?;
    }
    Ok(Json(DisplayPowerDto { on: request.on }))
}
