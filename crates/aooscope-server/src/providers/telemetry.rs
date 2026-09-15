use super::http::HttpClient;
use aooscope_config::AppPaths;
use aooscope_types::{ProviderSecrets, Settings, StateDocument};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::PathBuf};

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

async fn collect_proxmox(
    settings: &Settings,
    secrets: &ProviderSecrets,
    paths: Option<&AppPaths>,
) -> Result<Value, String> {
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
    let mut smart = Vec::with_capacity(disk_data.len());
    for disk in &disk_data {
        let value = if let Some(devpath) = disk.get("devpath").and_then(Value::as_str)
            && let Ok(value) = client
                .get_json(
                    &config.url,
                    &format!("/api2/json/nodes/{node}/disks/smart?disk={devpath}"),
                    &headers,
                )
                .await
        {
            value.get("data").cloned().unwrap_or(value)
        } else {
            Value::Null
        };
        smart.push(value);
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
        normalize_local_sysfs, normalize_ollama, normalize_proxmox, proxmox_auth_values,
        proxmox_authorization, proxmox_ca_pem,
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
        assert_eq!(value["smart"][0]["temperature_c"], 31);
        assert!(value["smart"][1]["health"].is_null());
        assert_eq!(value["smart"][2]["temperature_c"], 33);
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
