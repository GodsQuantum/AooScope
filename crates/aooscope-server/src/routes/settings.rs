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

fn coordinated_write<F, G>(
    paths: (&std::path::Path, &std::path::Path),
    previous: (&Settings, &ProviderSecrets),
    current: (&Settings, &ProviderSecrets),
    mut write_settings: F,
    mut write_secrets: G,
) -> Result<(), aooscope_config::ConfigError>
where
    F: FnMut(&std::path::Path, &Settings) -> Result<(), aooscope_config::ConfigError>,
    G: FnMut(&std::path::Path, &ProviderSecrets) -> Result<(), aooscope_config::ConfigError>,
{
    write_settings(paths.0, current.0)?;
    if let Err(error) = write_secrets(paths.1, current.1) {
        if let Err(rollback) = write_settings(paths.0, previous.0) {
            tracing::error!(%rollback, "settings rollback failed after provider secret write failure");
        }
        if let Err(rollback) = write_secrets(paths.1, previous.1) {
            tracing::error!(%rollback, "provider secret rollback failed after provider secret write failure");
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::coordinated_write;
    use aooscope_types::{ProviderSecrets, Settings};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn rolls_back_settings_and_private_secrets_when_private_commit_fails() {
        let old = Settings {
            display: Default::default(),
            providers: Default::default(),
            extra: Default::default(),
        };
        let new = Settings {
            display: aooscope_types::DisplaySettings {
                brightness: 42,
                ..Default::default()
            },
            ..old.clone()
        };
        let old_secrets: ProviderSecrets = serde_json::from_value(serde_json::json!({
            "proxmox": { "api_token": "old-token" }
        }))
        .unwrap();
        let new_secrets: ProviderSecrets = serde_json::from_value(serde_json::json!({
            "proxmox": { "api_token": "new-token" }
        }))
        .unwrap();
        let root = std::env::temp_dir().join(format!("aooscope-settings-{}", uuid::Uuid::new_v4()));
        let settings_path = root.join("settings.json");
        let secrets_path = root.join("private/providers.json");
        std::fs::create_dir_all(secrets_path.parent().unwrap()).unwrap();
        aooscope_config::atomic_write_json(&settings_path, &old).unwrap();
        aooscope_config::atomic_write_private_json(&secrets_path, &old_secrets).unwrap();
        let mut writes = 0;
        coordinated_write(
            (&settings_path, &secrets_path),
            (&old, &old_secrets),
            (&new, &new_secrets),
            aooscope_config::atomic_write_json,
            |path, value| {
                writes += 1;
                if writes == 1 {
                    aooscope_config::atomic_write_private_json(path, value).unwrap();
                    return Err(aooscope_config::ConfigError::NoParent(path.to_path_buf()));
                }
                aooscope_config::atomic_write_private_json(path, value)
            },
        )
        .unwrap_err();
        let written: Settings =
            serde_json::from_slice(&std::fs::read(&settings_path).unwrap()).unwrap();
        assert_eq!(written, old);
        let written_secrets: ProviderSecrets =
            serde_json::from_slice(&std::fs::read(&secrets_path).unwrap()).unwrap();
        assert_eq!(written_secrets, old_secrets);
        #[cfg(unix)]
        assert_eq!(
            std::fs::metadata(&secrets_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

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
    let previous_settings = load_settings(&state.paths)?;
    let mut secrets: ProviderSecrets = load_provider_secrets(&state.paths).unwrap_or_default();
    let previous_secrets = secrets.clone();
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
                            if provider == "beszel" && *field == "username" {
                                "email"
                            } else {
                                *field
                            }
                            .into(),
                            secret,
                        );
                    } else if secret.is_null() || secret.as_str() == Some("") {
                        bucket.remove(*field);
                        if provider == "beszel" && *field == "username" {
                            bucket.remove("email");
                        }
                    }
                }
            }
        }
    }
    let settings: Settings = serde_json::from_value(value)
        .map_err(|_| crate::error::ApiError::BadRequest("invalid settings"))?;
    coordinated_write(
        (&state.paths.settings(), &state.paths.provider_secrets()),
        (&previous_settings, &previous_secrets),
        (&settings, &secrets),
        atomic_write_json,
        atomic_write_private_json,
    )?;
    Ok(Json(public_settings(settings, &secrets)))
}
