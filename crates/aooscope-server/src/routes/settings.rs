use crate::{
    dto::{PublicSettingsDto, public_settings},
    error::ApiResult,
    state::AppState,
};
use aooscope_config::{load_provider_secrets, load_settings};
use axum::{Json, extract::State};

#[utoipa::path(get, path = "/api/settings", responses((status = 200, body = PublicSettingsDto)))]
pub async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<PublicSettingsDto>> {
    let settings = load_settings(&state.paths)?;
    let secrets = load_provider_secrets(&state.paths).unwrap_or_default();
    Ok(Json(public_settings(settings, &secrets)))
}
