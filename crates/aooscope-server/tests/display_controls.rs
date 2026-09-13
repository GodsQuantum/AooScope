use aooscope_config::AppPaths;
use aooscope_display::{DisplayCapabilities, DisplayDriver, SimulatedDisplayDriver};
use aooscope_server::{AppState, app};
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tower::ServiceExt;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1")
}

#[tokio::test]
async fn driverless_display_does_not_advertise_power_control() {
    let state = AppState::new(AppPaths::new(fixture_root()));
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/api/display/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "width": 960, "height": 376, "native_brightness": false, "power_control": false,
            "power_on": false
        })
    );

    let response = app(AppState::new(AppPaths::new(fixture_root())))
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/display/power")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"on":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn display_capabilities_are_available_without_hardware() {
    let state = AppState::new(AppPaths::new(fixture_root()))
        .with_display_driver(SimulatedDisplayDriver::new());
    let response = app(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/display/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "width": 960, "height": 376, "native_brightness": false, "power_control": true,
            "power_on": false
        })
    );
}

#[tokio::test]
async fn power_route_uses_the_display_worker() {
    let state = AppState::new(AppPaths::new(fixture_root()))
        .with_display_driver(SimulatedDisplayDriver::new());
    let response = app(state.clone())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/display/power")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"on":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(value, serde_json::json!({"on": false}));

    let response = app(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/display/capabilities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(value["power_on"], false);

    let response = app(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/display/power")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"on":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn luminance_updates_only_display_brightness() {
    let root =
        std::env::temp_dir().join(format!("aooscope-display-luminance-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("settings.json"),
        br#"{"display":{"brightness":100,"unknown_display":true},"providers":{"x":{"url":"keep"}},"unknown":42}"#,
    )
    .unwrap();
    let paths = AppPaths::new(&root);
    let state = AppState::new(paths.clone()).with_display_driver(SimulatedDisplayDriver::new());
    let response = app(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/display/luminance")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"value":37}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(paths.settings()).unwrap()).unwrap(),
        serde_json::json!({"display":{"brightness":37,"unknown_display":true},"providers":{"x":{"url":"keep"}},"unknown":42})
    );
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn luminance_rejects_values_outside_range() {
    let state = AppState::new(AppPaths::new(fixture_root()))
        .with_display_driver(SimulatedDisplayDriver::new());
    let response = app(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/display/luminance")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"value":101}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn simulated_driver_keeps_native_brightness_disabled() {
    assert_eq!(
        SimulatedDisplayDriver::new().capabilities(),
        DisplayCapabilities::default()
    );
    assert!(!DisplayCapabilities::default().native_brightness);
}
