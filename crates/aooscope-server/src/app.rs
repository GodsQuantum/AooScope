use crate::{assets, routes, state::AppState};
use axum::{
    Router,
    http::{HeaderName, StatusCode},
    routing::{delete, get, post, put},
};
use std::time::Duration;
use tower_http::{
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub fn app(state: AppState) -> Router {
    let request_id = HeaderName::from_static("x-request-id");
    Router::new()
        .route("/api/health", get(routes::health::get_health))
        .route("/api/status", get(routes::status::get_status))
        .route("/api/settings", get(routes::settings::get_settings))
        .route(
            "/api/pages",
            get(routes::pages::get_pages).post(routes::designer::create_page),
        )
        .route(
            "/api/pages/{id}",
            get(routes::designer::get_page)
                .put(routes::designer::update_page)
                .delete(routes::designer::delete_page),
        )
        .route(
            "/api/pages/{id}/duplicate",
            post(routes::designer::duplicate_page),
        )
        .route(
            "/api/pages/{id}/restore",
            post(routes::designer::restore_page),
        )
        .route(
            "/api/media",
            get(routes::designer::get_media).post(routes::designer::upload_media),
        )
        .route(
            "/api/media/presets/orbit",
            post(routes::designer::create_orbit_preset),
        )
        .route(
            "/api/media/{id}",
            delete(routes::designer::delete_media).put(routes::designer::replace_media),
        )
        .route("/api/media/{id}/file", get(routes::designer::media_file))
        .route("/api/carousel", put(routes::designer::update_carousel))
        .route("/api/sensors", get(routes::designer::get_sensors))
        .route("/api/preview", post(routes::designer::preview))
        .route("/api/apply", post(routes::designer::apply))
        .route("/api/metrics", get(routes::catalog::get_metrics))
        .route(
            "/api/providers/catalog",
            get(routes::catalog::get_provider_catalog),
        )
        .route("/api/events", get(routes::events::get_events))
        .route("/", get(assets::index))
        .route("/{*path}", get(assets::asset))
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(RequestBodyLimitLayer::new(64 * 1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .layer(TraceLayer::new_for_http())
}
