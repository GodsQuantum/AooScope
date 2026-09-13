use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

pub const CANVAS_WIDTH: i32 = 960;
pub const CANVAS_HEIGHT: i32 = 376;
pub const ALLOWED_LAYER_TYPES: &[&str] = &[
    "text",
    "value",
    "bar",
    "gauge",
    "ring",
    "badge",
    "image",
    "sparkline",
    "animation",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PagesDocument {
    pub schema_version: u32,
    pub revision: u64,
    #[serde(default)]
    pub carousel: Vec<String>,
    #[serde(default)]
    pub pages: BTreeMap<String, Page>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Page {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub duration: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    pub revision: u64,
    #[serde(default)]
    pub background: PageBackground,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PageBackground {
    #[serde(default = "default_background")]
    pub color: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for PageBackground {
    fn default() -> Self {
        Self {
            color: default_background(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Layer {
    pub id: String,
    #[serde(rename = "type")]
    pub layer_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub z: i32,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub clip: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn default_true() -> bool {
    true
}
fn default_background() -> String {
    "#071019".into()
}
fn default_opacity() -> f64 {
    1.0
}

pub fn validate_document(doc: &PagesDocument) -> Result<(), Vec<String>> {
    let mut issues = Vec::new();
    if doc.schema_version != 1 {
        issues.push("unsupported schema_version".into());
    }
    if doc.revision < 1 {
        issues.push("invalid document revision".into());
    }
    let mut page_ids = std::collections::HashSet::new();
    for (id, page) in &doc.pages {
        if page.id != *id {
            issues.push(format!("{id}: page id mismatch"));
        }
        if page.name.trim().is_empty() {
            issues.push(format!("{id}: invalid name"));
        }
        if !(2..=120).contains(&page.duration) {
            issues.push(format!("{id}: invalid duration"));
        }
        if !page_ids.insert(&page.id) {
            issues.push("duplicate page ids".into());
        }
        if !valid_color(&page.background.color) {
            issues.push(format!("{id}/background/color: invalid color"));
        }
        let mut layer_ids = std::collections::HashSet::new();
        for layer in &page.layers {
            let prefix = format!("{id}/{}", layer.id);
            if layer.id.is_empty() {
                issues.push(format!("{id}: layer missing id"));
            }
            if !layer_ids.insert(&layer.id) {
                issues.push(format!("{id}: duplicate layer id {}", layer.id));
            }
            if !ALLOWED_LAYER_TYPES.contains(&layer.layer_type.as_str()) {
                issues.push(format!("{prefix}: unknown layer type"));
            }
            if layer.width == 0 || layer.height == 0 {
                issues.push(format!("{prefix}: non-positive geometry"));
            }
            if !layer.clip
                && (layer.x < 0
                    || layer.y < 0
                    || layer.x + layer.width as i32 > CANVAS_WIDTH
                    || layer.y + layer.height as i32 > CANVAS_HEIGHT)
            {
                issues.push(format!("{prefix}: geometry outside canvas"));
            }
            if !(-10000..=10000).contains(&layer.z) {
                issues.push(format!("{prefix}: invalid z"));
            }
            if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
                issues.push(format!("{prefix}: invalid opacity"));
            }
            for (key, value) in &layer.extra {
                if (key == "color" || key.ends_with("_color"))
                    && (!value.is_string() || !valid_color(value.as_str().unwrap_or_default()))
                {
                    issues.push(format!("{prefix}/{key}: invalid color"));
                }
            }
        }
    }
    let mut carousel_ids = std::collections::HashSet::new();
    for id in &doc.carousel {
        if !carousel_ids.insert(id) {
            issues.push("carousel contains duplicate page ids".into());
        }
        if !doc.pages.contains_key(id) {
            issues.push(format!("carousel references missing pages: {id}"));
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn valid_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}
