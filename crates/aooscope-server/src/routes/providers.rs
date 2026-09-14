use crate::{dto::ProviderStatusDto, error::ApiResult, state::AppState};
use aooscope_config::{load_provider_secrets, load_settings, load_state};
use aooscope_types::Settings;
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;

fn statuses(state: &Value, settings: &Settings) -> Vec<ProviderStatusDto> {
    let runtime = state.pointer("/meta/providers").and_then(Value::as_object);
    [
        "local",
        "proxmox",
        "beszel",
        "jellyfin",
        "silo",
        "radarr",
        "sonarr",
        "qbittorrent",
        "immich",
        "ollama",
    ]
    .into_iter()
    .map(|id| {
        let configured = provider_configured(id, settings);
        let enabled = provider_enabled(id, settings);
        let value = runtime.and_then(|items| items.get(id));
        ProviderStatusDto {
            id: id.into(),
            configured,
            enabled,
            online: configured
                && value
                    .and_then(|v| v.get("online"))
                    .and_then(Value::as_bool)
                    .unwrap_or(id == "local"),
            last_success: configured
                .then(|| {
                    value
                        .and_then(|v| v.get("last_success"))
                        .and_then(Value::as_i64)
                })
                .flatten(),
            error: configured
                .then(|| {
                    value
                        .and_then(|v| v.get("error"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .flatten(),
        }
    })
    .collect()
}

fn provider_configured(id: &str, settings: &Settings) -> bool {
    id == "local"
        || settings
            .providers
            .get(id)
            .is_some_and(|provider| !provider.url.trim().is_empty())
}

fn provider_enabled(id: &str, settings: &Settings) -> bool {
    id == "local"
        || settings
            .providers
            .get(id)
            .is_some_and(|provider| provider.enabled)
}

#[utoipa::path(get, path = "/api/providers/status", responses((status = 200, body = [ProviderStatusDto])))]
pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<Vec<ProviderStatusDto>>> {
    Ok(Json(statuses(
        &serde_json::to_value(load_state(&state.paths)?).unwrap_or_default(),
        &load_settings(&state.paths)?,
    )))
}

#[utoipa::path(post, path = "/api/providers/{id}/test", params(("id" = String, Path)), responses((status = 200, body = ProviderStatusDto)))]
pub async fn test_provider(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProviderStatusDto>> {
    let settings = load_settings(&state.paths)?;
    let secrets = load_provider_secrets(&state.paths).unwrap_or_default();
    let mut result = ProviderStatusDto {
        id: id.clone(),
        configured: provider_configured(&id, &settings),
        enabled: provider_enabled(&id, &settings),
        online: false,
        last_success: None,
        error: None,
    };
    if result.configured {
        let mut test_settings = settings.clone();
        if let Some(provider) = test_settings.providers.get_mut(&id) {
            provider.enabled = true;
        }
        match crate::providers::telemetry::test_provider(&id, &test_settings, &secrets).await {
            Ok(()) => {
                result.online = true;
                result.last_success = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|value| value.as_secs() as i64)
                        .unwrap_or_default(),
                );
            }
            Err(error) => result.error = Some(error),
        }
    } else {
        result.error = Some("not configured".into());
    }
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::{provider_configured, provider_enabled};
    use aooscope_types::Settings;
    use serde_json::json;

    #[test]
    fn configured_and_enabled_are_independent() {
        let settings: Settings = serde_json::from_value(json!({
            "providers": {"ollama": {"enabled": false, "url": "http://ollama.test"}}
        }))
        .unwrap();
        assert!(provider_configured("ollama", &settings));
        assert!(!provider_enabled("ollama", &settings));
    }
}
