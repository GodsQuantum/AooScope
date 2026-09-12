use aooscope_config::AppPaths;
use aooscope_server::{AppState, app};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tower::ServiceExt;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1")
}

async fn get_json(path: &str) -> (StatusCode, Value) {
    let router = app(AppState::new(AppPaths::new(fixture_root())));
    let response = router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn health_and_status_match_reference_shape() {
    let (status, health) = get_json("/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["ok"], true);
    assert!(health["version"].is_string());

    let (_, value) = get_json("/api/status").await;
    assert_eq!(value["brightness"], 100);
    assert_eq!(value["native_brightness"], false);
    assert_eq!(value["device_present"], false);
    assert_eq!(value["updated_unix"], 1789200000);
}

#[tokio::test]
async fn settings_mask_secrets_and_pages_follow_carousel_order() {
    let (_, settings) = get_json("/api/settings").await;
    assert_eq!(settings["display"]["timezone"], "UTC");
    assert_eq!(settings["providers"]["proxmox"]["secret_set"], true);
    assert!(settings["providers"]["proxmox"].get("api_token").is_none());
    assert_eq!(settings["providers"]["ollama"]["secret_set"], false);

    let (_, pages) = get_json("/api/pages").await;
    assert_eq!(pages["schema_version"], 1);
    assert_eq!(pages["revision"], 3);
    assert_eq!(pages["carousel"][0], "page-home");
    assert_eq!(pages["pages"][0]["id"], "page-home");
    assert_eq!(pages["pages"][0]["name"], "Home");
    assert_eq!(pages["pages"][0]["template_id"], "home");
}
