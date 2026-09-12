use crate::{
    dto::{
        HealthDto, PageSummaryDto, PagesListDto, PublicProviderDto, PublicSettingsDto, StatusDto,
    },
    routes,
};
use aooscope_types::{DisplaySettings, ScheduleRule};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::get_health,
        routes::status::get_status,
        routes::settings::get_settings,
        routes::pages::get_pages
    ),
    components(schemas(
        HealthDto, StatusDto, PublicSettingsDto, PublicProviderDto,
        PagesListDto, PageSummaryDto, DisplaySettings, ScheduleRule
    )),
    tags((name = "aooscope", description = "AooScope compatibility API"))
)]
pub struct ApiDoc;
