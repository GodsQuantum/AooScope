use crate::{
    dto::{
        HealthDto, PageSummaryDto, PagesListDto, PublicProviderDto, PublicSettingsDto, StatusDto,
    },
    routes,
};
use aooscope_types::{
    DisplaySettings, MetricCatalogDto, MetricDescriptor, ProviderCatalogDto, ProviderDescriptor,
    ScheduleRule, WidgetKind,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::get_health,
        routes::status::get_status,
        routes::settings::get_settings,
        routes::pages::get_pages,
        routes::catalog::get_metrics,
        routes::catalog::get_provider_catalog
    ),
    components(schemas(
        HealthDto, StatusDto, PublicSettingsDto, PublicProviderDto,
        PagesListDto, PageSummaryDto, DisplaySettings, ScheduleRule,
        MetricCatalogDto, MetricDescriptor, ProviderCatalogDto, ProviderDescriptor, WidgetKind
    )),
    tags((name = "aooscope", description = "AooScope compatibility API"))
)]
pub struct ApiDoc;
