use aooscope_config::AppPaths;
use aooscope_server::{AppState, app};
use aooscope_types::StateDocument;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tower::ServiceExt;

fn state_with_disks(count: usize) -> StateDocument {
    let disks = (0..count)
        .map(|index| {
            serde_json::json!({
                "name": format!("Disk {index}"),
                "path": format!("/dev/sd{}", (b'a' + index as u8) as char),
                "size": 1_000_000_000_u64,
                "used": 250_000_000_u64,
                "avail": 750_000_000_u64,
                "usage_pct": 25.0,
                "type": "ssd"
            })
        })
        .collect::<Vec<_>>();
    let smart = (0..count)
        .map(|index| serde_json::json!({"temperature_c": 30 + index, "health": "PASSED"}))
        .collect::<Vec<_>>();
    serde_json::from_value(serde_json::json!({
        "pve": {"disks": disks, "smart": smart}
    }))
    .unwrap()
}

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

#[tokio::test]
async fn storage_metric_keeps_legacy_plural_disk_name_binding() {
    let metrics = aooscope_server::metrics::metric_catalog(&state_with_disks(1));
    assert!(
        metrics
            .iter()
            .any(|metric| metric.id == "aooscope_pve_disks_0_name")
    );
    assert!(
        !metrics
            .iter()
            .any(|metric| metric.id == "aooscope_pve_disk_0_name")
    );
}

#[test]
fn storage_metric_count_matches_actual_disk_inventory() {
    for (disk_count, expected_metric_count) in [(0, 0), (1, 7), (8, 56), (10, 70)] {
        let metrics = aooscope_server::metrics::metric_catalog(&state_with_disks(disk_count));
        let storage_count = metrics
            .iter()
            .filter(|metric| metric.category == "Stockage")
            .count();
        assert_eq!(storage_count, expected_metric_count, "{disk_count} disks");

        if disk_count == 8 {
            assert!(
                metrics
                    .iter()
                    .any(|metric| metric.id == "aooscope_pve_disks_7_size_bytes")
            );
        }
    }
}
