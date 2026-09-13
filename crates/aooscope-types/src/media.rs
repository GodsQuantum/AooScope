use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MediaDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub assets: BTreeMap<String, MediaAsset>,
    #[serde(default)]
    pub presets: BTreeMap<String, MediaPreset>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MediaPreset {
    pub id: String,
    pub name: String,
    pub source_asset_id: String,
    pub settings: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MediaAsset {
    pub id: String,
    pub name: String,
    pub stored_name: String,
    pub kind: String,
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    pub size: u64,
    pub sha256: String,
    pub revision: u64,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
