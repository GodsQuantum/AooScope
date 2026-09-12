use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Settings {
    #[serde(default)]
    pub display: DisplaySettings,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderSettings>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DisplaySettings {
    #[serde(default = "default_brand")]
    pub brand: String,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub schedule_enabled: bool,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_schedule")]
    pub schedule: Vec<ScheduleRule>,
    #[serde(default = "default_switch_seconds")]
    pub switch_seconds: u32,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ScheduleRule {
    pub start: String,
    pub end: String,
    pub brightness: u8,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProviderSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_verify_tls")]
    pub verify_tls: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn default_brand() -> String {
    "AOOSCOPE".into()
}
fn default_brightness() -> u8 {
    100
}
fn default_timezone() -> String {
    "UTC".into()
}
fn default_switch_seconds() -> u32 {
    8
}
fn default_verify_tls() -> bool {
    true
}
fn default_schedule() -> Vec<ScheduleRule> {
    vec![ScheduleRule {
        start: "22:00".into(),
        end: "08:00".into(),
        brightness: 70,
        extra: BTreeMap::new(),
    }]
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            brand: default_brand(),
            brightness: 100,
            schedule_enabled: false,
            timezone: default_timezone(),
            schedule: default_schedule(),
            switch_seconds: 8,
            extra: BTreeMap::new(),
        }
    }
}
