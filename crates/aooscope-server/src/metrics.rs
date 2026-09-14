use aooscope_types::{MetricDescriptor, ProviderDescriptor, StateDocument, WidgetKind};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
struct MetricDef {
    id: &'static str,
    pointer: &'static str,
    label: &'static str,
    provider_id: &'static str,
    provider_name: &'static str,
    category: &'static str,
    value_type: &'static str,
    unit: &'static str,
    min: Option<f64>,
    max: Option<f64>,
    demo: Option<fn() -> Value>,
    widgets: &'static [WidgetKind],
}

const NUMERIC: &[WidgetKind] = &[
    WidgetKind::Value,
    WidgetKind::Gauge,
    WidgetKind::Ring,
    WidgetKind::Bar,
];
const TEXT: &[WidgetKind] = &[WidgetKind::Text, WidgetKind::Value, WidgetKind::Badge];
const VALUE_ONLY: &[WidgetKind] = &[WidgetKind::Value, WidgetKind::Text];

fn demo_42() -> Value {
    json!(42)
}
fn demo_52() -> Value {
    json!(52)
}
fn demo_68() -> Value {
    json!(68)
}
fn demo_8() -> Value {
    json!(8)
}
fn demo_speed() -> Value {
    json!(44_000_000)
}
fn demo_media_title() -> Value {
    json!("Dune: Part Two")
}
fn demo_media_headline() -> Value {
    json!("JELLYFIN · LECTURE")
}

const CORE: &[MetricDef] = &[
    MetricDef {
        id: "aooscope_pve_cpu_pct",
        pointer: "/pve/cpu_pct",
        label: "Utilisation CPU",
        provider_id: "proxmox",
        provider_name: "Proxmox",
        category: "CPU",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_42),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_pve_memory_pct",
        pointer: "/pve/memory_pct",
        label: "Mémoire utilisée",
        provider_id: "proxmox",
        provider_name: "Proxmox",
        category: "Mémoire",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_52),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_pve_guests_running",
        pointer: "/pve/guests_running",
        label: "Invités actifs",
        provider_id: "proxmox",
        provider_name: "Proxmox",
        category: "Virtualisation",
        value_type: "number",
        unit: "",
        min: Some(0.0),
        max: None,
        demo: Some(demo_8),
        widgets: VALUE_ONLY,
    },
    MetricDef {
        id: "aooscope_hardware_cpu_temp_c",
        pointer: "/hardware/cpu_temp_c",
        label: "Température CPU",
        provider_id: "local",
        provider_name: "Hardware local",
        category: "CPU",
        value_type: "number",
        unit: "°C",
        min: Some(0.0),
        max: Some(120.0),
        demo: Some(demo_52),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_hardware_gpu_busy_pct",
        pointer: "/hardware/gpu_busy_pct",
        label: "Utilisation GPU",
        provider_id: "local",
        provider_name: "Hardware local",
        category: "GPU",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_42),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_hardware_gpu_gtt_pct",
        pointer: "/hardware/gpu_gtt_pct",
        label: "Mémoire GPU partagée",
        provider_id: "local",
        provider_name: "Hardware local",
        category: "GPU",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_52),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_hardware_gpu_temp_c",
        pointer: "/hardware/gpu_temp_c",
        label: "Température GPU",
        provider_id: "local",
        provider_name: "Hardware local",
        category: "GPU",
        value_type: "number",
        unit: "°C",
        min: Some(0.0),
        max: Some(120.0),
        demo: Some(demo_52),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_hardware_gpu_vram_pct",
        pointer: "/hardware/gpu_vram_pct",
        label: "VRAM utilisée",
        provider_id: "local",
        provider_name: "Hardware local",
        category: "GPU",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_52),
        widgets: NUMERIC,
    },
];

const MEDIA: &[MetricDef] = &[
    MetricDef {
        id: "aooscope_media_display_headline",
        pointer: "/media/display/headline",
        label: "État du média",
        provider_id: "media",
        provider_name: "Media agrégé",
        category: "Lecture",
        value_type: "string",
        unit: "",
        min: None,
        max: None,
        demo: Some(demo_media_headline),
        widgets: TEXT,
    },
    MetricDef {
        id: "aooscope_media_display_title_short",
        pointer: "/media/display/title",
        label: "Titre",
        provider_id: "media",
        provider_name: "Media agrégé",
        category: "Lecture",
        value_type: "string",
        unit: "",
        min: None,
        max: None,
        demo: Some(demo_media_title),
        widgets: TEXT,
    },
    MetricDef {
        id: "aooscope_media_display_progress_pct",
        pointer: "/media/display/progress_pct",
        label: "Progression du média",
        provider_id: "media",
        provider_name: "Media agrégé",
        category: "Lecture",
        value_type: "number",
        unit: "%",
        min: Some(0.0),
        max: Some(100.0),
        demo: Some(demo_68),
        widgets: NUMERIC,
    },
    MetricDef {
        id: "aooscope_media_display_eta_minutes",
        pointer: "/media/display/eta_minutes",
        label: "Temps restant",
        provider_id: "media",
        provider_name: "Media agrégé",
        category: "Téléchargement",
        value_type: "number",
        unit: "min",
        min: Some(0.0),
        max: None,
        demo: Some(demo_8),
        widgets: VALUE_ONLY,
    },
    MetricDef {
        id: "aooscope_media_display_speed_bytes_s",
        pointer: "/media/display/speed_bytes_s",
        label: "Débit de téléchargement",
        provider_id: "media",
        provider_name: "Media agrégé",
        category: "Téléchargement",
        value_type: "number",
        unit: "B/s",
        min: Some(0.0),
        max: None,
        demo: Some(demo_speed),
        widgets: VALUE_ONLY,
    },
];

pub fn metric_catalog(state: &StateDocument) -> Vec<MetricDescriptor> {
    let doc = serde_json::to_value(state).unwrap_or_else(|_| json!({}));
    let media_online = doc
        .pointer("/media/display/mode")
        .and_then(Value::as_str)
        .is_some_and(|mode| !matches!(mode, "offline" | "idle"));
    CORE.iter()
        .chain(MEDIA.iter())
        .map(|def| {
            let value = doc.pointer(def.pointer).cloned().filter(|v| !v.is_null());
            let online = if def.provider_id == "media" {
                media_online && value.is_some()
            } else {
                value.is_some()
            };
            MetricDescriptor {
                id: def.id.to_owned(),
                label: def.label.to_owned(),
                provider_id: def.provider_id.to_owned(),
                provider_name: def.provider_name.to_owned(),
                category: def.category.to_owned(),
                value_type: def.value_type.to_owned(),
                unit: def.unit.to_owned(),
                min: def.min,
                max: def.max,
                value,
                demo_value: def.demo.map(|make| make()),
                online,
                recommended_widgets: def.widgets.to_vec(),
            }
        })
        .chain(storage_metrics(&doc))
        .collect()
}

fn storage_metrics(doc: &Value) -> impl Iterator<Item = MetricDescriptor> + '_ {
    (0..6).flat_map(move |index| {
        let temp_id = format!("aooscope_pve_smart_{index}_temperature_c");
        let health_id = format!("aooscope_pve_smart_{index}_health");
        let name_id = format!("aooscope_pve_disks_{index}_name");
        let name = doc.pointer(&format!("/pve/disks/{index}/name")).cloned();
        let temp = doc
            .pointer(&format!("/pve/smart/{index}/temperature_c"))
            .cloned();
        let health = doc.pointer(&format!("/pve/smart/{index}/health")).cloned();
        [
            MetricDescriptor {
                id: name_id,
                label: format!("Disque {} · nom", index + 1),
                provider_id: "proxmox".into(),
                provider_name: "Proxmox".into(),
                category: "Stockage".into(),
                value_type: "string".into(),
                unit: String::new(),
                min: None,
                max: None,
                online: name.is_some(),
                value: name,
                demo_value: Some(json!(format!("Disk {}", index + 1))),
                recommended_widgets: TEXT.to_vec(),
            },
            MetricDescriptor {
                id: temp_id,
                label: format!("Disque {} · température", index + 1),
                provider_id: "proxmox".into(),
                provider_name: "Proxmox".into(),
                category: "Stockage".into(),
                value_type: "number".into(),
                unit: "°C".into(),
                min: Some(0.0),
                max: Some(100.0),
                online: temp.is_some(),
                value: temp,
                demo_value: Some(json!(38 + index)),
                recommended_widgets: NUMERIC.to_vec(),
            },
            MetricDescriptor {
                id: health_id,
                label: format!("Disque {} · état SMART", index + 1),
                provider_id: "proxmox".into(),
                provider_name: "Proxmox".into(),
                category: "Stockage".into(),
                value_type: "string".into(),
                unit: String::new(),
                min: None,
                max: None,
                online: health.is_some(),
                value: health,
                demo_value: Some(json!("OK")),
                recommended_widgets: TEXT.to_vec(),
            },
        ]
    })
}

fn provider(
    id: &str,
    name: &str,
    icon: &str,
    categories: &[&str],
    credential_fields: &[&str],
) -> ProviderDescriptor {
    ProviderDescriptor {
        id: id.to_owned(),
        name: name.to_owned(),
        icon: icon.to_owned(),
        categories: categories.iter().map(|value| (*value).to_owned()).collect(),
        credential_fields: credential_fields
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

pub fn provider_catalog() -> Vec<ProviderDescriptor> {
    vec![
        provider(
            "proxmox",
            "Proxmox",
            "proxmox",
            &["CPU", "Mémoire", "Virtualisation", "Stockage"],
            &["token_id", "token_secret"],
        ),
        provider(
            "local",
            "Hardware local",
            "hardware",
            &["CPU", "GPU", "Températures"],
            &[],
        ),
        provider(
            "beszel",
            "Beszel",
            "beszel",
            &["Monitoring", "Historique"],
            &["email", "password"],
        ),
        provider(
            "jellyfin",
            "Jellyfin",
            "jellyfin",
            &["Lecture", "Bibliothèque"],
            &["api_key"],
        ),
        provider(
            "silo",
            "Silo",
            "silo",
            &["Lecture", "Bibliothèque"],
            &["api_key"],
        ),
        provider(
            "radarr",
            "Radarr",
            "radarr",
            &["Téléchargement", "Bibliothèque"],
            &["api_key"],
        ),
        provider(
            "sonarr",
            "Sonarr",
            "sonarr",
            &["Téléchargement", "Bibliothèque"],
            &["api_key"],
        ),
        provider(
            "qbittorrent",
            "qBittorrent",
            "qbittorrent",
            &["Téléchargement"],
            &["username", "password"],
        ),
        provider(
            "immich",
            "Immich",
            "immich",
            &["Photos", "Souvenirs"],
            &["api_key"],
        ),
        provider("ollama", "Ollama", "ollama", &["IA", "Modèles"], &[]),
    ]
}
