use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    Text,
    Value,
    Bar,
    Gauge,
    Ring,
    Badge,
    Sparkline,
    Image,
    Animation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MetricDescriptor {
    pub id: String,
    pub label: String,
    pub provider_id: String,
    pub provider_name: String,
    pub category: String,
    pub value_type: String,
    pub unit: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub value: Option<Value>,
    pub demo_value: Option<Value>,
    pub online: bool,
    pub recommended_widgets: Vec<WidgetKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProviderDescriptor {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub categories: Vec<String>,
    pub credential_fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MetricCatalogDto {
    pub metrics: Vec<MetricDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProviderCatalogDto {
    pub providers: Vec<ProviderDescriptor>,
}
