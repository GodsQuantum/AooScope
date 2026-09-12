use crate::{routes, state::AppState};
use axum::{
    Router,
    http::{HeaderName, StatusCode},
    routing::get,
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
        .route("/api/pages", get(routes::pages::get_pages))
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
