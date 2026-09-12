use aooscope_config::AppPaths;
use aooscope_server::{AppState, StatusDto, app};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tower::ServiceExt;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1")
}

fn state() -> AppState {
    AppState::new(AppPaths::new(fixture_root())).with_device("/definitely/missing")
}

#[tokio::test]
async fn serves_embedded_frontend_with_cache_policy() {
    let router = app(state());
    let response = router
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
    assert!(response.headers().get(header::ETAG).is_some());
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    let start = html.find("/_app/immutable/").expect("hashed Vite asset");
    let tail = &html[start..];
    let end = tail.find(['\"', '\'']).unwrap_or(tail.len());
    let asset_path = &tail[..end];

    let asset = router
        .oneshot(
            Request::builder()
                .uri(asset_path)
                .header(header::ACCEPT_ENCODING, "br, gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(
        asset.headers()[header::CACHE_CONTROL],
        "public,max-age=31536000,immutable"
    );
    let encoding = asset
        .headers()
        .get(header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok());
    assert!(matches!(encoding, Some("br") | Some("gzip") | None));
}

#[tokio::test]
async fn sse_emits_status_after_watch_update() {
    let state = state();
    let router = app(state.clone());
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/event-stream"
    );

    state.publish_status(StatusDto {
        version: "test".into(),
        brightness: 37,
        native_brightness: false,
        device_present: false,
        updated_unix: Some(123),
    });

    let mut stream = response.into_body().into_data_stream();
    let chunk = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("SSE event timeout")
        .expect("SSE stream ended")
        .expect("SSE body error");
    let text = String::from_utf8_lossy(&chunk);
    assert!(text.contains("event: status"), "{text}");
    assert!(text.contains("\"brightness\":37"), "{text}");
}
