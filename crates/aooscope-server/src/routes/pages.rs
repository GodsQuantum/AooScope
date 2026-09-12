use crate::{
    dto::{PageSummaryDto, PagesListDto},
    error::ApiResult,
    state::AppState,
};
use aooscope_config::load_pages;
use axum::{Json, extract::State};

#[utoipa::path(get, path = "/api/pages", responses((status = 200, body = PagesListDto)))]
pub async fn get_pages(State(state): State<AppState>) -> ApiResult<Json<PagesListDto>> {
    let doc = load_pages(&state.paths)?;
    let pages = doc
        .carousel
        .iter()
        .filter_map(|id| doc.pages.get(id))
        .map(PageSummaryDto::from)
        .collect();
    Ok(Json(PagesListDto {
        schema_version: doc.schema_version,
        revision: doc.revision,
        carousel: doc.carousel,
        pages,
    }))
}
