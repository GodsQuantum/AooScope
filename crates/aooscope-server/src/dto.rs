use aooscope_types::{DisplaySettings, Page, ProviderSettings, Settings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, ToSchema)]
pub struct DisplayCapabilitiesDto {
    pub width: u32,
    pub height: u32,
    pub native_brightness: bool,
    pub power_control: bool,
    pub power_on: bool,
}

impl From<aooscope_display::DisplayCapabilities> for DisplayCapabilitiesDto {
    fn from(value: aooscope_display::DisplayCapabilities) -> Self {
        Self {
            width: value.width,
            height: value.height,
            native_brightness: value.native_brightness,
            power_control: value.power_control,
            power_on: false,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DisplayPowerRequest {
    pub on: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DisplayPowerDto {
    pub on: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthDto {
    pub ok: bool,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct StatusDto {
    pub version: String,
    pub brightness: u8,
    pub native_brightness: bool,
    pub device_present: bool,
    pub updated_unix: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicSettingsDto {
    pub display: DisplaySettings,
    pub providers: BTreeMap<String, PublicProviderDto>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicProviderDto {
    pub enabled: bool,
    pub url: String,
    pub verify_tls: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub secret_set: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderStatusDto {
    pub id: String,
    pub configured: bool,
    pub enabled: bool,
    pub online: bool,
    pub last_success: Option<i64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PageSummaryDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub duration: u32,
    pub template_id: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PagesListDto {
    pub schema_version: u32,
    pub revision: u64,
    pub carousel: Vec<String>,
    pub pages: Vec<PageSummaryDto>,
}
fn secret_fields(name: &str) -> &'static [&'static str] {
    match name {
        "proxmox" => &["api_token", "token_id", "token_secret"],
        "beszel" => &["email", "username", "password"],
        "jellyfin" | "silo" | "radarr" | "sonarr" | "immich" => &["api_key"],
        "qbittorrent" => &["username", "password"],
        _ => &[],
    }
}

pub fn public_settings(
    settings: Settings,
    secrets: &BTreeMap<String, BTreeMap<String, Value>>,
) -> PublicSettingsDto {
    let providers = settings
        .providers
        .into_iter()
        .map(|(name, provider)| {
            let public = public_provider(&name, provider, secrets.get(&name));
            (name, public)
        })
        .collect();
    PublicSettingsDto {
        display: settings.display,
        providers,
        extra: settings.extra,
    }
}
fn public_provider(
    name: &str,
    provider: ProviderSettings,
    secrets: Option<&BTreeMap<String, Value>>,
) -> PublicProviderDto {
    let mut extra = provider.extra;
    for field in secret_fields(name) {
        extra.remove(*field);
        extra.remove(&format!("clear_{field}"));
    }
    let secret_set = secret_fields(name).iter().any(|field| {
        secrets
            .and_then(|bucket| bucket.get(*field))
            .is_some_and(|value| match value {
                Value::String(text) => !text.is_empty(),
                Value::Null => false,
                _ => true,
            })
    });
    PublicProviderDto {
        enabled: provider.enabled,
        url: provider.url,
        verify_tls: provider.verify_tls,
        node: provider.node,
        system: provider.system,
        secret_set,
        extra,
    }
}

impl From<&Page> for PageSummaryDto {
    fn from(page: &Page) -> Self {
        Self {
            id: page.id.clone(),
            name: page.name.clone(),
            enabled: page.enabled,
            duration: page.duration,
            template_id: page.template_id.clone(),
            revision: page.revision,
        }
    }
}
