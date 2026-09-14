use super::http::HttpClient;
use aooscope_types::{ProviderSecrets, Settings, StateDocument};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

pub fn normalize_proxmox(value: &Value) -> Value {
    let status = value.get("status").unwrap_or(value);
    let memory = status.get("memory").unwrap_or(&Value::Null);
    let total = memory.get("total").and_then(Value::as_f64).unwrap_or(0.0);
    let used = memory.get("used").and_then(Value::as_f64).unwrap_or(0.0);
    let guests = value
        .get("guests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let disks = value
        .get("disks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let smart = value
        .get("smart")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({
        "cpu_pct": status.get("cpu").and_then(Value::as_f64).map(|v| v * 100.0),
        "memory_pct": (total > 0.0).then(|| used * 100.0 / total),
        "memory_used_bytes": used,
        "memory_total_bytes": total,
        "guests_running": guests.iter().filter(|guest| guest.get("status").and_then(Value::as_str) == Some("running")).count(),
        "guests_total": guests.len(),
        "disks": disks.iter().map(|disk| json!({
            "name": disk.get("model").or_else(|| disk.get("devpath")).cloned().unwrap_or(Value::Null),
            "path": disk.get("devpath"),
            "health": disk.get("health")
        })).collect::<Vec<_>>(),
        "smart": smart.iter().map(|disk| json!({
            "health": disk.get("health"),
            "temperature_c": disk.get("temperature").or_else(|| disk.get("temperature_c"))
        })).collect::<Vec<_>>()
    })
}

pub fn normalize_local_sysfs(temperatures: &[(&str, &str)], gpu: &[(&str, &str)]) -> Value {
    let temp = |name: &str| {
        temperatures
            .iter()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse::<f64>().ok())
            .map(|value| value / 1000.0)
    };
    let number = |name: &str| {
        gpu.iter()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse::<f64>().ok())
    };
    let pct = |used: Option<f64>, total: Option<f64>| match (used, total) {
        (Some(used), Some(total)) if total > 0.0 => Some(used * 100.0 / total),
        _ => None,
    };
    json!({
        "cpu_temp_c": temp("cpu"), "gpu_temp_c": temp("gpu"), "gpu_busy_pct": number("busy"),
        "gpu_vram_pct": pct(number("vram_used"), number("vram_total")),
        "gpu_gtt_pct": pct(number("gtt_used"), number("gtt_total")),
        "gpu_vram_used_bytes": number("vram_used"), "gpu_vram_total_bytes": number("vram_total"),
        "gpu_gtt_used_bytes": number("gtt_used"), "gpu_gtt_total_bytes": number("gtt_total")
    })
}

pub fn normalize_ollama(value: &Value) -> Value {
    json!({"version": value.get("version"), "models": value.get("models").and_then(Value::as_array).map(Vec::len), "running": value.get("running").and_then(Value::as_array).map(Vec::len)})
}

fn configured<'a>(
    settings: &'a Settings,
    name: &str,
) -> Option<&'a aooscope_types::ProviderSettings> {
    settings
        .providers
        .get(name)
        .filter(|p| p.enabled && !p.url.trim().is_empty())
}

fn secret<'a>(secrets: &'a ProviderSecrets, provider: &str, names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|name| {
        secrets
            .get(provider)?
            .get(*name)?
            .as_str()
            .filter(|value| !value.is_empty())
    })
}

pub fn proxmox_authorization(
    api_token: Option<&str>,
    token_id: Option<&str>,
    token_secret: Option<&str>,
) -> String {
    match (token_id, token_secret) {
        (Some(id), Some(secret)) => format!("PVEAPIToken={id}:{secret}"),
        _ => format!("PVEAPIToken={}", api_token.unwrap_or("")),
    }
}

fn status(configured: bool, online: bool, error: Option<&str>) -> Value {
    json!({"configured": configured, "enabled": configured, "online": online, "last_success": online.then(chrono_like_now), "error": error})
}

fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or_default()
}

async fn provider_json(
    settings: &Settings,
    name: &str,
    path: &str,
    headers: Vec<(&str, &str)>,
) -> Result<Value, String> {
    let config = configured(settings, name).ok_or_else(|| "not configured".to_owned())?;
    HttpClient::new(config.verify_tls)
        .map_err(|_| "client unavailable".to_owned())?
        .get_json(&config.url, path, &headers)
        .await
        .map_err(|error| error.to_string())
}

async fn collect_proxmox(settings: &Settings, secrets: &ProviderSecrets) -> Result<Value, String> {
    let config = configured(settings, "proxmox").ok_or_else(|| "not configured".to_owned())?;
    let node = config
        .node
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");
    let authorization = proxmox_authorization(
        secret(secrets, "proxmox", &["api_token"]),
        secret(secrets, "proxmox", &["token_id"]),
        secret(secrets, "proxmox", &["token_secret"]),
    );
    let headers = [("Authorization", authorization.as_str())];
    let client = HttpClient::new(config.verify_tls).map_err(|_| "client unavailable".to_owned())?;
    let status = client
        .get_json(
            &config.url,
            &format!("/api2/json/nodes/{node}/status"),
            &headers,
        )
        .await
        .map_err(|e| e.to_string())?;
    let qemu = client
        .get_json(
            &config.url,
            &format!("/api2/json/nodes/{node}/qemu"),
            &headers,
        )
        .await
        .unwrap_or(json!([]));
    let lxc = client
        .get_json(
            &config.url,
            &format!("/api2/json/nodes/{node}/lxc"),
            &headers,
        )
        .await
        .unwrap_or(json!([]));
    let disks = client
        .get_json(
            &config.url,
            &format!("/api2/json/nodes/{node}/disks/list"),
            &headers,
        )
        .await
        .unwrap_or(json!([]));
    let mut guests = qemu
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    guests.extend(
        lxc.get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let disk_data = disks
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut smart = Vec::new();
    for disk in &disk_data {
        if let Some(devpath) = disk.get("devpath").and_then(Value::as_str)
            && let Ok(value) = client
                .get_json(
                    &config.url,
                    &format!("/api2/json/nodes/{node}/disks/smart?disk={devpath}"),
                    &headers,
                )
                .await
        {
            smart.push(value.get("data").cloned().unwrap_or(value));
        }
    }
    Ok(normalize_proxmox(
        &json!({"status": status.get("data").cloned().unwrap_or(status), "guests": guests, "disks": disk_data, "smart": smart}),
    ))
}

fn local_sysfs() -> Value {
    let mut temperature_values = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            let kind = fs::read_to_string(path.join("type"))
                .unwrap_or_default()
                .to_ascii_lowercase();
            let key = if kind.contains("gpu") {
                Some("gpu")
            } else if kind.contains("cpu") || kind.contains("package") || kind.contains("x86") {
                Some("cpu")
            } else {
                None
            };
            if let Some(key) = key
                && let Ok(value) = fs::read_to_string(path.join("temp"))
            {
                temperature_values.push((key, value));
            }
        }
    }
    let temperatures = temperature_values
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();
    let mut gpu = Vec::new();
    for (key, path) in [
        ("busy", "/sys/class/drm/card0/device/gpu_busy_percent"),
        (
            "vram_used",
            "/sys/class/drm/card0/device/mem_info_vram_used",
        ),
        (
            "vram_total",
            "/sys/class/drm/card0/device/mem_info_vram_total",
        ),
        ("gtt_used", "/sys/class/drm/card0/device/mem_info_gtt_used"),
        (
            "gtt_total",
            "/sys/class/drm/card0/device/mem_info_gtt_total",
        ),
    ] {
        if let Ok(value) = fs::read_to_string(path) {
            gpu.push((key, value.trim().to_owned()));
        }
    }
    normalize_local_sysfs(
        &temperatures,
        &gpu.iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect::<Vec<_>>(),
    )
}

async fn collect_remote(
    settings: &Settings,
    secrets: &ProviderSecrets,
    name: &str,
) -> Result<Value, String> {
    match name {
        "beszel" => collect_beszel(settings, secrets).await,
        "immich" => provider_json(
            settings,
            name,
            "/api/server/ping",
            vec![(
                "x-api-key",
                secret(secrets, name, &["api_key"]).unwrap_or(""),
            )],
        )
        .await
        .map(|value| json!({"ping": value})),
        "ollama" => {
            let version = provider_json(settings, name, "/api/version", Vec::new()).await?;
            let models = provider_json(settings, name, "/api/tags", Vec::new())
                .await
                .unwrap_or(json!({"models": []}));
            let running = provider_json(settings, name, "/api/ps", Vec::new())
                .await
                .unwrap_or(json!({"models": []}));
            Ok(normalize_ollama(
                &json!({"version": version.get("version"), "models": models.get("models"), "running": running.get("models")}),
            ))
        }
        _ => Err("unsupported telemetry provider".into()),
    }
}

async fn collect_beszel(settings: &Settings, secrets: &ProviderSecrets) -> Result<Value, String> {
    let config = configured(settings, "beszel").ok_or_else(|| "not configured".to_owned())?;
    let email = secret(secrets, "beszel", &["email", "username"])
        .ok_or_else(|| "missing email".to_owned())?;
    let password =
        secret(secrets, "beszel", &["password"]).ok_or_else(|| "missing password".to_owned())?;
    let client = HttpClient::new(config.verify_tls).map_err(|_| "client unavailable".to_owned())?;
    let auth = client
        .post_json(
            &config.url,
            "/api/collections/users/auth-with-password",
            &json!({"identity": email, "password": password}),
            &[],
        )
        .await
        .map_err(|error| error.to_string())?;
    let token = auth
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "authentication token missing".to_owned())?;
    let authorization = format!("Bearer {token}");
    let systems = client
        .get_json(
            &config.url,
            "/api/collections/systems/records",
            &[("Authorization", authorization.as_str())],
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(
        json!({"systems": systems.get("items").or_else(|| systems.get("data")).and_then(Value::as_array).map(Vec::len).unwrap_or(0)}),
    )
}

pub async fn test_provider(
    name: &str,
    settings: &Settings,
    secrets: &ProviderSecrets,
) -> Result<(), String> {
    match name {
        "local" => Ok(()),
        "proxmox" => collect_proxmox(settings, secrets).await.map(|_| ()),
        "beszel" | "immich" | "ollama" => collect_remote(settings, secrets, name).await.map(|_| ()),
        "jellyfin" | "silo" | "radarr" | "sonarr" | "qbittorrent" => {
            crate::providers::media::collect_provider(name, settings, secrets)
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        _ => Err("unsupported provider".into()),
    }
}

pub async fn collect_telemetry_state(
    settings: &Settings,
    secrets: &ProviderSecrets,
) -> StateDocument {
    let mut state = StateDocument {
        hardware: Some(local_sysfs()),
        ..StateDocument::default()
    };
    let mut providers = BTreeMap::new();
    providers.insert("local", status(true, true, None));
    if configured(settings, "proxmox").is_some() {
        match collect_proxmox(settings, secrets).await {
            Ok(value) => {
                state.pve = Some(value);
                providers.insert("proxmox", status(true, true, None));
            }
            Err(error) => {
                providers.insert("proxmox", status(true, false, Some(&error)));
            }
        }
    } else {
        providers.insert("proxmox", status(false, false, None));
    }
    for name in ["beszel", "immich", "ollama"] {
        if configured(settings, name).is_some() {
            match collect_remote(settings, secrets, name).await {
                Ok(value) => {
                    state.extra.insert(name.into(), value);
                    providers.insert(name, status(true, true, None));
                }
                Err(error) => {
                    providers.insert(name, status(true, false, Some(&error)));
                }
            }
        } else {
            providers.insert(name, status(false, false, None));
        }
    }
    state.meta = Some(json!({"updated_unix": chrono_like_now(), "providers": providers}));
    state
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_local_sysfs, normalize_ollama, normalize_proxmox, proxmox_authorization,
    };
    use serde_json::json;

    #[test]
    fn proxmox_normalization_keeps_guest_disk_and_smart_semantics() {
        let value = normalize_proxmox(
            &json!({"status":{"cpu":0.25,"memory":{"used":512,"total":1024}},"guests":[{"status":"running"},{"status":"stopped"}],"disks":[{"devpath":"/dev/sda","model":"Disk A"}],"smart":[{"health":"PASSED","temperature":37}]}),
        );
        assert_eq!(value["cpu_pct"], 25.0);
        assert_eq!(value["memory_pct"], 50.0);
        assert_eq!(value["guests_running"], 1);
        assert_eq!(value["disks"][0]["name"], "Disk A");
        assert_eq!(value["smart"][0]["health"], "PASSED");
    }

    #[test]
    fn local_sysfs_normalization_uses_millidegrees_and_percent_memory() {
        let value = normalize_local_sysfs(
            &[("cpu", "52000"), ("gpu", "61000")],
            &[
                ("busy", "42"),
                ("vram_used", "256"),
                ("vram_total", "1024"),
                ("gtt_used", "128"),
                ("gtt_total", "512"),
            ],
        );
        assert_eq!(value["cpu_temp_c"], 52.0);
        assert_eq!(value["gpu_temp_c"], 61.0);
        assert_eq!(value["gpu_busy_pct"], 42.0);
        assert_eq!(value["gpu_vram_pct"], 25.0);
        assert_eq!(value["gpu_gtt_pct"], 25.0);
    }

    #[test]
    fn provider_responses_are_normalized_without_credentials() {
        let value = normalize_ollama(
            &json!({"version":"0.3.0","models":[{"name":"llama3"}],"running":[{"name":"llama3"}]}),
        );
        assert_eq!(value["version"], "0.3.0");
        assert_eq!(value["models"], 1);
        assert_eq!(value["running"], 1);
        assert!(!serde_json::to_string(&value).unwrap().contains("secret"));
    }

    #[test]
    fn proxmox_authorization_supports_legacy_and_split_credentials() {
        assert_eq!(
            proxmox_authorization(Some("legacy"), None, None),
            "PVEAPIToken=legacy"
        );
        assert_eq!(
            proxmox_authorization(None, Some("user@pam!token"), Some("secret")),
            "PVEAPIToken=user@pam!token:secret"
        );
    }
}
