use crate::{
    dto::{PublicSettingsDto, public_settings},
    error::ApiResult,
    state::AppState,
};
use aooscope_config::{atomic_write_json, atomic_write_private_json};
use aooscope_config::{load_provider_secrets, load_settings};
use aooscope_types::{ProviderSecrets, Settings};
use axum::{Json, extract::State};
use serde_json::Value;

#[utoipa::path(get, path = "/api/settings", responses((status = 200, body = PublicSettingsDto)))]
pub async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<PublicSettingsDto>> {
    let settings = load_settings(&state.paths)?;
    let secrets = load_provider_secrets(&state.paths).unwrap_or_default();
    Ok(Json(public_settings(settings, &secrets)))
}

#[utoipa::path(put, path = "/api/settings", request_body = Value, responses((status = 200, body = PublicSettingsDto)))]
pub async fn put_settings(
    State(state): State<AppState>,
    Json(mut value): Json<Value>,
) -> ApiResult<Json<PublicSettingsDto>> {
    let _guard = state.settings_lock.lock().expect("settings lock poisoned");
    let mut secrets: ProviderSecrets = load_provider_secrets(&state.paths).unwrap_or_default();
    if let Some(providers) = value.get_mut("providers").and_then(Value::as_object_mut) {
        for (provider, config) in providers {
            let Some(config) = config.as_object_mut() else {
                continue;
            };
            config.remove("secret_set");
            let fields = match provider.as_str() {
                "proxmox" => &["api_token", "token_id", "token_secret"][..],
                "beszel" => &["email", "username", "password"],
                "jellyfin" | "silo" | "radarr" | "sonarr" | "immich" => &["api_key"],
                "qbittorrent" => &["username", "password"],
                _ => &[][..],
            };
            let bucket = secrets.entry(provider.clone()).or_default();
            for field in fields {
                if let Some(secret) = config.remove(*field) {
                    if secret.as_str().is_some_and(|value| !value.is_empty()) {
                        bucket.insert(
                            if *field == "username" {
                                "email"
                            } else {
                                *field
                            }
                            .into(),
                            secret,
                        );
                    } else if secret.is_null() || secret.as_str() == Some("") {
                        bucket.remove(*field);
                        if *field == "username" {
                            bucket.remove("email");
                        }
                    }
                }
            }
        }
    }
    let settings: Settings = serde_json::from_value(value)
        .map_err(|_| crate::error::ApiError::BadRequest("invalid settings"))?;
    atomic_write_json(&state.paths.settings(), &settings)?;
    atomic_write_private_json(&state.paths.provider_secrets(), &secrets)?;
    Ok(Json(public_settings(settings, &secrets)))
}
