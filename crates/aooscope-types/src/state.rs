use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StateDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pve: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub type ProviderSecrets = BTreeMap<String, BTreeMap<String, Value>>;
