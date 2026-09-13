use crate::{error::ApiResult, metrics, state::AppState};
use aooscope_config::load_state;
use aooscope_types::{MetricCatalogDto, ProviderCatalogDto};
use axum::{Json, extract::State};

#[utoipa::path(
    get,
    path = "/api/metrics",
    responses((status = 200, body = MetricCatalogDto))
)]
pub async fn get_metrics(State(state): State<AppState>) -> ApiResult<Json<MetricCatalogDto>> {
    let state_doc = load_state(&state.paths)?;
    Ok(Json(MetricCatalogDto {
        metrics: metrics::metric_catalog(&state_doc),
    }))
}

#[utoipa::path(
    get,
    path = "/api/providers/catalog",
    responses((status = 200, body = ProviderCatalogDto))
)]
pub async fn get_provider_catalog() -> Json<ProviderCatalogDto> {
    Json(ProviderCatalogDto {
        providers: metrics::provider_catalog(),
    })
}
