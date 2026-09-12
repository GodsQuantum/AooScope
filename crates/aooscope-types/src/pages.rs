use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

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
