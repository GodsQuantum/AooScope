use aooscope_types::StateDocument;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct StorageDevice {
    pub index: usize,
    pub path: String,
    pub label: String,
    pub kind: String,
    pub total_bytes: u64,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub usage_pct: Option<f64>,
    pub temperature_c: Option<f64>,
    pub health: Option<String>,
}

fn nonnegative_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn bytes(value: Option<&Value>) -> Option<u64> {
    nonnegative_number(value).map(|value| value as u64)
}

pub fn inventory(state: &StateDocument) -> Vec<StorageDevice> {
    let Some(pve) = state.pve.as_ref() else {
        return Vec::new();
    };
    let disks = pve
        .get("disks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let smart = pve
        .get("smart")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    disks
        .iter()
        .enumerate()
        .map(|(index, disk)| {
            let smart = smart.get(index).unwrap_or(&Value::Null);
            let total_bytes = bytes(disk.get("size")).unwrap_or_default();
            let used_bytes = bytes(disk.get("used"));
            let free_bytes = bytes(disk.get("avail"))
                .or_else(|| used_bytes.and_then(|used| total_bytes.checked_sub(used)));
            let usage_pct = nonnegative_number(disk.get("usage_pct"))
                .or_else(|| {
                    used_bytes.zip(free_bytes).and_then(|(used, free)| {
                        let total = used.saturating_add(free);
                        (total > 0).then(|| used as f64 * 100.0 / total as f64)
                    })
                })
                .or_else(|| {
                    used_bytes.and_then(|used| {
                        (total_bytes > 0).then(|| used as f64 * 100.0 / total_bytes as f64)
                    })
                });
            StorageDevice {
                index,
                path: disk
                    .get("path")
                    .or_else(|| disk.get("devpath"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                label: disk
                    .get("name")
                    .or_else(|| disk.get("model"))
                    .or_else(|| disk.get("path"))
                    .or_else(|| disk.get("devpath"))
                    .and_then(Value::as_str)
                    .unwrap_or("Disk")
                    .to_owned(),
                kind: disk
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("disk")
                    .to_owned(),
                total_bytes,
                used_bytes,
                free_bytes,
                usage_pct,
                temperature_c: smart
                    .get("temperature_c")
                    .or_else(|| smart.get("temperature"))
                    .and_then(Value::as_f64),
                health: disk
                    .get("health")
                    .or_else(|| smart.get("health"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            }
        })
        .collect()
}
