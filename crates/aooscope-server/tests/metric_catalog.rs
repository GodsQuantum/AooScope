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
async fn metrics_are_human_readable_grouped_and_widget_aware() {
    let (status, body) = get_json("/api/metrics").await;
    assert_eq!(status, StatusCode::OK);
    let metrics = body["metrics"].as_array().unwrap();
    let cpu = metrics
        .iter()
        .find(|m| m["id"] == "aooscope_pve_cpu_pct")
        .unwrap();
    assert_eq!(cpu["label"], "Utilisation CPU");
    assert_eq!(cpu["provider_name"], "Proxmox");
    assert_eq!(cpu["category"], "CPU");
    assert_eq!(cpu["unit"], "%");
    assert_eq!(cpu["value"], 21.5);
    assert_eq!(cpu["online"], true);
    assert!(
        cpu["recommended_widgets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "gauge")
    );
}

#[tokio::test]
async fn offline_media_metrics_keep_demo_values_and_provider_catalog_is_explicit() {
    let (_, body) = get_json("/api/metrics").await;
    let metrics = body["metrics"].as_array().unwrap();
    let progress = metrics
        .iter()
        .find(|m| m["id"] == "aooscope_media_display_progress_pct")
        .unwrap();
    assert_eq!(progress["online"], false);
    assert_eq!(progress["demo_value"], 68);
    assert_eq!(progress["provider_name"], "Media agrégé");

    let (status, providers) = get_json("/api/providers/catalog").await;
    assert_eq!(status, StatusCode::OK);
    let list = providers["providers"].as_array().unwrap();
    assert!(
        list.iter()
            .any(|p| p["id"] == "jellyfin" && p["name"] == "Jellyfin")
    );
    assert!(
        list.iter()
            .any(|p| p["id"] == "sonarr" && p["name"] == "Sonarr")
    );
    assert!(
        list.iter()
            .any(|p| p["id"] == "local" && p["name"] == "Hardware local")
    );
}
