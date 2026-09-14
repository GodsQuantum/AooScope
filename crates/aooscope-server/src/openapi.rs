use crate::{
    dto::{
        DisplayCapabilitiesDto, DisplayPowerDto, DisplayPowerRequest, HealthDto, PageSummaryDto,
        PagesListDto, ProviderStatusDto, PublicProviderDto, PublicSettingsDto, StatusDto,
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
        routes::catalog::get_provider_catalog,
        routes::display::get_capabilities,
        routes::display::set_power,
        routes::display::set_luminance,
        routes::settings::put_settings,
        routes::providers::get_status,
        routes::providers::test_provider
    ),
    components(schemas(
        HealthDto, StatusDto, PublicSettingsDto, PublicProviderDto,
        PagesListDto, PageSummaryDto, DisplaySettings, ScheduleRule,
        DisplayCapabilitiesDto, DisplayPowerRequest, DisplayPowerDto, ProviderStatusDto,
        routes::display::DisplayLuminanceRequest,
        MetricCatalogDto, MetricDescriptor, ProviderCatalogDto, ProviderDescriptor, WidgetKind
    )),
    tags((name = "aooscope", description = "AooScope compatibility API"))
)]
pub struct ApiDoc;
