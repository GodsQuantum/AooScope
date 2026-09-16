use super::http::HttpClient;
use aooscope_config::AppPaths;
use aooscope_types::{ProviderSecrets, Settings, StateDocument};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn storage_sort_key(disk: &Value) -> (String, String) {
    (
        disk.get("path")
            .or_else(|| disk.get("devpath"))
            .or_else(|| disk.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        disk.get("name")
            .or_else(|| disk.get("model"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    )
}

fn temperature_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| {
            value
                .as_str()
                .and_then(|raw| raw.trim().parse::<f64>().ok())
        })
        .or_else(|| value.get("current").and_then(temperature_number))
}

fn smart_raw_temperature(value: &Value) -> Option<f64> {
    temperature_number(value).or_else(|| {
        value.as_str().and_then(|raw| {
            raw.split_whitespace().next().and_then(|part| {
                part.trim_matches(|ch: char| !ch.is_ascii_digit() && ch != '.' && ch != '-')
                    .parse::<f64>()
                    .ok()
            })
        })
    })
}

fn smart_temperature_c(smart: &Value) -> Option<f64> {
    for key in ["temperature", "temperature_c"] {
        if let Some(value) = smart.get(key).and_then(temperature_number) {
            return Some(value);
        }
    }

    if let Some(attributes) = smart.get("attributes").and_then(Value::as_array) {
        for wanted in ["194", "190"] {
            if let Some(value) = attributes.iter().find_map(|attribute| {
                let id = attribute.get("id").and_then(|value| {
                    value
                        .as_str()
                        .map(str::trim)
                        .map(str::to_owned)
                        .or_else(|| value.as_u64().map(|id| id.to_string()))
                })?;
                (id == wanted)
                    .then(|| attribute.get("raw"))
                    .flatten()
                    .and_then(smart_raw_temperature)
            }) {
                return Some(value);
            }
        }
    }

    smart.get("text").and_then(Value::as_str).and_then(|text| {
        text.lines().find_map(|line| {
            let (label, value) = line.split_once(':')?;
            (label.trim().eq_ignore_ascii_case("Temperature"))
                .then(|| value.split_whitespace().next())
                .flatten()
                .and_then(|value| value.parse::<f64>().ok())
        })
    })
}

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
    let mut storage = disks
        .into_iter()
        .enumerate()
        .map(|(index, disk)| (disk, smart.get(index).cloned().unwrap_or(Value::Null)))
        .collect::<Vec<_>>();
    storage.sort_by_key(|(disk, _)| storage_sort_key(disk));
    json!({
        "cpu_pct": status.get("cpu").and_then(Value::as_f64).map(|v| v * 100.0),
        "memory_pct": (total > 0.0).then(|| used * 100.0 / total),
        "memory_used_bytes": used,
        "memory_total_bytes": total,
        "guests_running": guests.iter().filter(|guest| guest.get("status").and_then(Value::as_str) == Some("running")).count(),
        "guests_total": guests.len(),
        "disks": storage.iter().map(|(disk, _)| {
            let size_value = disk.get("size").cloned().unwrap_or(Value::Null);
            let used_value = disk.get("used").cloned().unwrap_or(Value::Null);
            let avail_value = disk.get("avail").cloned().unwrap_or_else(|| {
                disk.get("size").and_then(Value::as_u64)
                    .zip(disk.get("used").and_then(Value::as_u64))
                    .and_then(|(size, used)| size.checked_sub(used))
                    .map(|free| json!(free))
                    .unwrap_or(Value::Null)
            });
            let used = used_value.as_f64();
            let avail = avail_value.as_f64();
            let usage_pct = disk.get("usage_pct").cloned().or_else(|| {
                used.zip(avail).and_then(|(used, avail)| {
                    let total = used + avail;
                    (total > 0.0).then(|| json!(used * 100.0 / total))
                })
            });
            json!({
                "name": disk
                    .get("name")
                    .or_else(|| disk.get("model"))
                    .or_else(|| disk.get("devpath"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "path": disk
                    .get("path")
                    .or_else(|| disk.get("devpath"))
                    .or_else(|| disk.get("id")),
                "devpath": disk
                    .get("devpath")
                    .or_else(|| disk.get("path"))
                    .or_else(|| disk.get("id")),
                "health": disk.get("health"),
                "size": size_value,
                "size_bytes": size_value,
                "type": disk.get("type"),
                "used": used_value,
                "used_bytes": used_value,
                "avail": avail_value,
                "free_bytes": avail_value,
                "usage_pct": usage_pct
            })
        }).collect::<Vec<_>>(),
        "smart": storage.iter().map(|(_, disk)| json!({
            "health": disk.get("health"),
            "temperature_c": smart_temperature_c(disk)
        })).collect::<Vec<_>>()
    })
}

pub fn normalize_local_sysfs(temperatures: &[(&str, &str)], gpu: &[(&str, &str)]) -> Value {
    let temp = |name: &str| {
        temperatures
            .iter()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.trim().parse::<f64>().ok())
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
        (Some(id), Some(secret)) => format!("PVEAPIToken={id}={secret}"),
        _ => format!("PVEAPIToken={}", api_token.unwrap_or("")),
    }
}

fn pve_private_path(paths: &AppPaths, env_name: &str, default_name: &str) -> PathBuf {
    std::env::var_os(env_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.root.join("private").join(default_name))
}

fn legacy_proxmox_credentials(paths: &AppPaths) -> Option<(String, String)> {
    let path = pve_private_path(paths, "PVE_TOKEN_FILE", "pve-token.json");
    let value: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let token_id = value
        .get("full-tokenid")
        .or_else(|| value.get("token_id"))
        .and_then(Value::as_str)?
        .to_owned();
    let token_secret = value
        .get("value")
        .or_else(|| value.get("token_secret"))
        .and_then(Value::as_str)?
        .to_owned();
    (!token_id.is_empty() && !token_secret.is_empty()).then_some((token_id, token_secret))
}

fn proxmox_auth_values(
    secrets: &ProviderSecrets,
    paths: Option<&AppPaths>,
) -> (Option<String>, Option<String>, Option<String>) {
    let api_token = secret(secrets, "proxmox", &["api_token"]).map(str::to_owned);
    let token_id = secret(secrets, "proxmox", &["token_id"]).map(str::to_owned);
    let token_secret = secret(secrets, "proxmox", &["token_secret"]).map(str::to_owned);
    if api_token.is_some() || (token_id.is_some() && token_secret.is_some()) {
        return (api_token, token_id, token_secret);
    }
    if let Some((legacy_id, legacy_secret)) = paths.and_then(legacy_proxmox_credentials) {
        return (None, Some(legacy_id), Some(legacy_secret));
    }
    (api_token, token_id, token_secret)
}

fn proxmox_ca_pem(paths: Option<&AppPaths>) -> Option<Vec<u8>> {
    let paths = paths?;
    let path = pve_private_path(paths, "PVE_CA_FILE", "pve-root-ca.pem");
    fs::read(path).ok().filter(|bytes| !bytes.is_empty())
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

fn disk_data_if_valid(value: &Value) -> Option<Vec<Value>> {
    value.get("data").and_then(Value::as_array).cloned()
}

async fn collect_proxmox(
    settings: &Settings,
    secrets: &ProviderSecrets,
    paths: Option<&AppPaths>,
) -> Result<(Value, bool), String> {
    let config = configured(settings, "proxmox").ok_or_else(|| "not configured".to_owned())?;
    let node = config
        .node
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");
    let (api_token, token_id, token_secret) = proxmox_auth_values(secrets, paths);
    let authorization = proxmox_authorization(
        api_token.as_deref(),
        token_id.as_deref(),
        token_secret.as_deref(),
    );
    let headers = [("Authorization", authorization.as_str())];
    let ca_pem = proxmox_ca_pem(paths);
    let client = HttpClient::new_with_ca(config.verify_tls, ca_pem.as_deref())
        .map_err(|_| "client unavailable".to_owned())?;
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
    let disks_result = client
        .get_json(
            &config.url,
            &format!("/api2/json/nodes/{node}/disks/list?skipsmart=1"),
            &headers,
        )
        .await;
    let (disk_data, mut storage_fresh) = match disks_result {
        Ok(disks) => match disk_data_if_valid(&disks) {
            Some(data) => (data, true),
            None => {
                tracing::warn!("proxmox disk inventory returned an invalid payload");
                (Vec::new(), false)
            }
        },
        Err(error) => {
            tracing::warn!(%error, "proxmox disk inventory temporarily unavailable");
            (Vec::new(), false)
        }
    };
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
    let mut smart = Vec::with_capacity(disk_data.len());
    for disk in &disk_data {
        let (value, fresh) = if let Some(devpath) = disk.get("devpath").and_then(Value::as_str) {
            match client
                .get_json(
                    &config.url,
                    &format!("/api2/json/nodes/{node}/disks/smart?disk={devpath}"),
                    &headers,
                )
                .await
            {
                Ok(value) => (value.get("data").cloned().unwrap_or(value), true),
                Err(error) => {
                    tracing::warn!(%error, %devpath, "proxmox SMART probe temporarily unavailable");
                    (Value::Null, false)
                }
            }
        } else {
            (Value::Null, false)
        };
        storage_fresh &= fresh;
        smart.push(value);
    }
    let mut normalized = normalize_proxmox(
        &json!({"status": status.get("data").cloned().unwrap_or(status), "guests": guests, "disks": disk_data, "smart": smart}),
    );
    if storage_fresh {
        apply_host_storage_overlay(&mut normalized, paths);
    }
    Ok((normalized, storage_fresh))
}

fn apply_host_storage_overlay(pve: &mut Value, paths: Option<&AppPaths>) {
    let Some(paths) = paths else {
        return;
    };
    let path = paths.root.join("private/host-storage.json");
    let Ok(bytes) = fs::read(&path) else {
        return;
    };
    let Ok(overlay) = serde_json::from_slice::<Value>(&bytes) else {
        tracing::warn!(path = %path.display(), "invalid host storage overlay");
        return;
    };
    let Some(devices) = overlay.get("devices").and_then(Value::as_array) else {
        return;
    };
    let Some(disks) = pve.get_mut("disks").and_then(Value::as_array_mut) else {
        return;
    };
    for disk in disks {
        let Some(devpath) = disk.get("devpath").and_then(Value::as_str) else {
            continue;
        };
        let Some(source) = devices
            .iter()
            .find(|item| item.get("devpath").and_then(Value::as_str) == Some(devpath))
        else {
            continue;
        };
        let Some(target) = disk.as_object_mut() else {
            continue;
        };
        let original_name = target.get("name").cloned();
        for key in [
            "filesystem_label",
            "mountpoint",
            "size",
            "used",
            "avail",
            "usage_pct",
        ] {
            if let Some(value) = source.get(key).filter(|value| !value.is_null()) {
                target.insert(key.to_owned(), value.clone());
            }
        }
        if let Some(display_name) = source
            .get("display_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            if !target.contains_key("model")
                && let Some(model) = original_name.filter(|value| !value.is_null())
            {
                target.insert("model".into(), model);
            }
            target.insert("display_name".into(), Value::from(display_name));
            target.insert("name".into(), Value::from(display_name));
        }
        if let Some(value) = target.get("size").cloned() {
            target.insert("size_bytes".into(), value);
        }
        if let Some(value) = target.get("used").cloned() {
            target.insert("used_bytes".into(), value);
        }
        if let Some(value) = target.get("avail").cloned() {
            target.insert("free_bytes".into(), value);
        }
    }
}

fn read_hwmon_temperatures(root: &Path) -> Vec<(String, String)> {
    let mut values = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return values;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = fs::read_to_string(path.join("name"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let key = if name.contains("amdgpu") || name.contains("radeon") {
            Some("gpu")
        } else if name.contains("k10temp")
            || name.contains("coretemp")
            || name.contains("zenpower")
            || name.contains("cpu")
            || name.contains("package")
            || name.contains("x86_pkg_temp")
        {
            Some("cpu")
        } else {
            None
        };
        if let Some(key) = key
            && let Ok(value) = fs::read_to_string(path.join("temp1_input"))
        {
            values.push((key.to_owned(), value));
        }
    }
    values
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
    let has_cpu = temperature_values.iter().any(|(key, _)| *key == "cpu");
    let has_gpu = temperature_values.iter().any(|(key, _)| *key == "gpu");
    if !has_cpu || !has_gpu {
        for (key, value) in read_hwmon_temperatures(Path::new("/sys/class/hwmon")) {
            if (key == "cpu" && !has_cpu) || (key == "gpu" && !has_gpu) {
                temperature_values.push((if key == "cpu" { "cpu" } else { "gpu" }, value));
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
    test_provider_with_paths(name, settings, secrets, None).await
}

pub async fn test_provider_with_paths(
    name: &str,
    settings: &Settings,
    secrets: &ProviderSecrets,
    paths: Option<&AppPaths>,
) -> Result<(), String> {
    match name {
        "local" => Ok(()),
        "proxmox" => collect_proxmox(settings, secrets, paths).await.map(|_| ()),
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
    collect_telemetry_state_with_paths(settings, secrets, None).await
}

pub async fn collect_telemetry_state_with_paths(
    settings: &Settings,
    secrets: &ProviderSecrets,
    paths: Option<&AppPaths>,
) -> StateDocument {
    let mut state = StateDocument {
        hardware: Some(local_sysfs()),
        ..StateDocument::default()
    };
    let mut providers = BTreeMap::new();
    providers.insert("local", status(true, true, None));
    if configured(settings, "proxmox").is_some() {
        match collect_proxmox(settings, secrets, paths).await {
            Ok((value, storage_fresh)) => {
                state.pve = Some(value);
                let mut provider = status(true, true, None);
                provider["storage_fresh"] = Value::Bool(storage_fresh);
                providers.insert("proxmox", provider);
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
        apply_host_storage_overlay, disk_data_if_valid, normalize_local_sysfs, normalize_ollama,
        normalize_proxmox, proxmox_auth_values, proxmox_authorization, proxmox_ca_pem,
        read_hwmon_temperatures, smart_temperature_c,
    };
    use aooscope_config::AppPaths;
    use aooscope_types::ProviderSecrets;
    use serde_json::json;
    use std::fs;

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
    fn malformed_successful_disk_payload_is_not_fresh() {
        assert!(disk_data_if_valid(&json!({"data": [{"devpath": "/dev/sda"}]})).is_some());
        assert!(disk_data_if_valid(&json!({"data": null})).is_none());
        assert!(disk_data_if_valid(&json!({"status": "ok"})).is_none());
    }

    #[test]
    fn host_storage_overlay_enriches_matching_devpath_without_reordering() {
        let root =
            std::env::temp_dir().join(format!("aooscope-host-storage-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("private")).unwrap();
        fs::write(root.join("private/host-storage.json"), serde_json::to_vec(&json!({"devices":[
            {"devpath":"/dev/sdb","display_name":"ARCHIVE DISK","filesystem_label":"archive_disk","size":3000,"used":2400,"avail":600,"usage_pct":80.0},
            {"devpath":"/dev/sda","display_name":"DATA PRIMARY","size":3000,"used":2700,"avail":300,"usage_pct":90.0}
        ]})).unwrap()).unwrap();
        let mut pve = normalize_proxmox(&json!({
            "disks":[{"devpath":"/dev/sda","path":"/dev/disk/by-id/disk-a","model":"A","size":3100},{"devpath":"/dev/sdb","model":"B","size":3100}],
            "smart":[{"health":"A-OK"},{"health":"B-OK"}]
        }));
        apply_host_storage_overlay(&mut pve, Some(&AppPaths::new(&root)));
        assert_eq!(pve["disks"][0]["devpath"], "/dev/sda");
        assert_eq!(pve["disks"][0]["path"], "/dev/disk/by-id/disk-a");
        assert_eq!(pve["disks"][0]["display_name"], "DATA PRIMARY");
        assert_eq!(pve["disks"][0]["name"], "DATA PRIMARY");
        assert_eq!(pve["disks"][0]["model"], "A");
        assert_eq!(pve["disks"][0]["usage_pct"], 90.0);
        assert_eq!(pve["disks"][1]["display_name"], "ARCHIVE DISK");
        assert_eq!(pve["smart"][0]["health"], "A-OK");
        assert_eq!(pve["smart"][1]["health"], "B-OK");

        fs::write(
            root.join("private/host-storage.json"),
            serde_json::to_vec(&json!({"devices":[
                {"devpath":"/dev/sda","display_name":"","filesystem_label":"data_primary"}
            ]}))
            .unwrap(),
        )
        .unwrap();
        let mut fallback = normalize_proxmox(&json!({
            "disks":[{"devpath":"/dev/sda","model":"A"}],
            "smart":[{"health":"A-OK"}]
        }));
        apply_host_storage_overlay(&mut fallback, Some(&AppPaths::new(&root)));
        assert!(fallback["disks"][0].get("display_name").is_none());
        assert_eq!(fallback["disks"][0]["name"], "A");
        assert_eq!(fallback["disks"][0]["filesystem_label"], "data_primary");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn proxmox_normalization_preserves_disk_size_type_and_calculates_usage() {
        let value = normalize_proxmox(&json!({
            "disks": [{
                "devpath": "/dev/nvme0n1",
                "model": "Disk A",
                "size": 1_000,
                "type": "ssd",
                "used": 250,
                "avail": 750
            }]
        }));

        assert_eq!(value["disks"][0]["size"], 1_000);
        assert_eq!(value["disks"][0]["type"], "ssd");
        assert_eq!(value["disks"][0]["used"], 250);
        assert_eq!(value["disks"][0]["avail"], 750);
        assert_eq!(value["disks"][0]["usage_pct"], 25.0);
        assert_eq!(value["disks"][0]["size_bytes"], 1_000);
        assert_eq!(value["disks"][0]["used_bytes"], 250);
        assert_eq!(value["disks"][0]["free_bytes"], 750);
    }

    #[test]
    fn smart_temperature_prefers_explicit_value_then_ata_194_then_190() {
        assert_eq!(smart_temperature_c(&json!({"temperature": 37})), Some(37.0));
        assert_eq!(
            smart_temperature_c(&json!({"temperature_c": 38})),
            Some(38.0)
        );
        assert_eq!(
            smart_temperature_c(&json!({"attributes": [
                {"id":"190","raw":"44 (Min/Max 30/60)"},
                {"id":"194","raw":"41 (0 13 0 0 0)"}
            ]})),
            Some(41.0)
        );
        assert_eq!(
            smart_temperature_c(&json!({"attributes": [
                {"id":" 190","raw":"43 (Min/Max 31/62)"}
            ]})),
            Some(43.0)
        );
    }

    #[test]
    fn smart_temperature_parses_proxmox_nvme_text() {
        let smart = json!({
            "health":"PASSED",
            "type":"text",
            "text":"SMART/Health Information\nTemperature:                        49 Celsius\nTemperature Sensor 1:               49 Celsius\n"
        });
        assert_eq!(smart_temperature_c(&smart), Some(49.0));
    }

    #[test]
    fn proxmox_normalization_preserves_smart_slots_when_a_request_failed() {
        let value = normalize_proxmox(&json!({
            "disks": [
                {"devpath":"/dev/sda"},
                {"devpath":"/dev/sdb"},
                {"devpath":"/dev/sdc"}
            ],
            "smart": [
                {"health":"PASSED","temperature":31},
                null,
                {"health":"PASSED","temperature":33}
            ]
        }));
        assert_eq!(value["smart"].as_array().unwrap().len(), 3);
        assert_eq!(value["smart"][0]["temperature_c"].as_f64(), Some(31.0));
        assert!(value["smart"][1]["health"].is_null());
        assert_eq!(value["smart"][2]["temperature_c"].as_f64(), Some(33.0));
    }

    #[test]
    fn proxmox_normalization_sorts_disks_and_smart_as_pairs() {
        let value = normalize_proxmox(&json!({
            "disks": [
                {"devpath":"/dev/sdb", "model":"B"},
                {"devpath":"/dev/sda", "model":"A"}
            ],
            "smart": [
                {"health":"B-HEALTH", "temperature":42},
                {"health":"A-HEALTH", "temperature":24}
            ]
        }));

        assert_eq!(value["disks"][0]["name"], "A");
        assert_eq!(value["disks"][1]["name"], "B");
        assert_eq!(value["smart"][0]["health"], "A-HEALTH");
        assert_eq!(value["smart"][1]["health"], "B-HEALTH");
    }

    #[test]
    fn proxmox_normalization_keeps_a_null_smart_slot_for_missing_result() {
        let value = normalize_proxmox(&json!({
            "disks": [{"devpath":"/dev/sda"}, {"devpath":"/dev/sdb"}],
            "smart": [{"health":"A-HEALTH"}]
        }));

        assert_eq!(value["smart"].as_array().unwrap().len(), 2);
        assert_eq!(value["smart"][0]["health"], "A-HEALTH");
        assert!(value["smart"][1]["health"].is_null());
    }

    #[test]
    fn hwmon_fallback_recognizes_cpu_and_amd_gpu_temperatures() {
        let root = std::env::temp_dir().join(format!("aooscope-hwmon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("hwmon0")).unwrap();
        fs::create_dir_all(root.join("hwmon1")).unwrap();
        fs::write(root.join("hwmon0/name"), "k10temp\n").unwrap();
        fs::write(root.join("hwmon0/temp1_input"), "86125\n").unwrap();
        fs::write(root.join("hwmon1/name"), "amdgpu\n").unwrap();
        fs::write(root.join("hwmon1/temp1_input"), "63000\n").unwrap();

        let values = read_hwmon_temperatures(&root);
        assert!(
            values
                .iter()
                .any(|(key, value)| key == "cpu" && value.trim() == "86125")
        );
        assert!(
            values
                .iter()
                .any(|(key, value)| key == "gpu" && value.trim() == "63000")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn local_sysfs_normalization_accepts_sysfs_newlines() {
        let value = normalize_local_sysfs(&[("cpu", "52000\n"), ("gpu", "61000\n")], &[]);
        assert_eq!(value["cpu_temp_c"], 52.0);
        assert_eq!(value["gpu_temp_c"], 61.0);
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
    fn legacy_proxmox_files_supply_auth_and_ca_without_copying_secrets() {
        let root =
            std::env::temp_dir().join(format!("aooscope-pve-legacy-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("private")).unwrap();
        fs::write(
            root.join("private/pve-token.json"),
            br#"{"full-tokenid":"user@pam!lcd","value":"secret-value"}"#,
        )
        .unwrap();
        fs::write(root.join("private/pve-root-ca.pem"), b"test-ca").unwrap();
        let paths = AppPaths::new(&root);
        let secrets = ProviderSecrets::new();

        let (api_token, token_id, token_secret) = proxmox_auth_values(&secrets, Some(&paths));
        assert_eq!(api_token, None);
        assert_eq!(token_id.as_deref(), Some("user@pam!lcd"));
        assert_eq!(token_secret.as_deref(), Some("secret-value"));
        assert_eq!(
            proxmox_ca_pem(Some(&paths)).as_deref(),
            Some(b"test-ca".as_slice())
        );
        assert!(!paths.provider_secrets().exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn provider_secret_store_takes_precedence_over_legacy_proxmox_file() {
        let root = std::env::temp_dir().join(format!(
            "aooscope-pve-secret-priority-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(root.join("private")).unwrap();
        fs::write(
            root.join("private/pve-token.json"),
            br#"{"full-tokenid":"legacy@pam!lcd","value":"legacy-secret"}"#,
        )
        .unwrap();
        let paths = AppPaths::new(&root);
        let secrets: ProviderSecrets = serde_json::from_value(json!({
            "proxmox": {"token_id":"admin@pam!lcd","token_secret":"admin-secret"}
        }))
        .unwrap();

        let (_, token_id, token_secret) = proxmox_auth_values(&secrets, Some(&paths));
        assert_eq!(token_id.as_deref(), Some("admin@pam!lcd"));
        assert_eq!(token_secret.as_deref(), Some("admin-secret"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn proxmox_authorization_supports_legacy_and_split_credentials() {
        assert_eq!(
            proxmox_authorization(Some("legacy"), None, None),
            "PVEAPIToken=legacy"
        );
        assert_eq!(
            proxmox_authorization(None, Some("user@pam!token"), Some("secret")),
            "PVEAPIToken=user@pam!token=secret"
        );
    }
}
