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

#[tokio::test]
async fn settings_put_round_trip_writes_private_secret_without_returning_it() {
    let root = std::env::temp_dir().join(format!("aooscope-settings-{}", uuid::Uuid::new_v4()));
    aooscope_server::bootstrap(&AppPaths::new(&root)).unwrap();
    let router = app(AppState::new(AppPaths::new(&root)));
    let request = Request::builder()
        .method("PUT")
        .uri("/api/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"display":{"brightness":73},"providers":{"immich":{"enabled":true,"url":"http://example.test","api_key":"hidden"}}}"#))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let returned: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(returned["display"]["brightness"], 73);
    assert!(returned["providers"]["immich"].get("api_key").is_none());
    assert_eq!(returned["providers"]["immich"]["secret_set"], true);
    let private: Value =
        serde_json::from_slice(&std::fs::read(root.join("private/providers.json")).unwrap())
            .unwrap();
    assert_eq!(private["immich"]["api_key"], "hidden");
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    assert_eq!(
        std::fs::metadata(root.join("private/providers.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn provider_status_reports_configured_media_providers() {
    let root =
        std::env::temp_dir().join(format!("aooscope-provider-status-{}", uuid::Uuid::new_v4()));
    aooscope_server::bootstrap(&AppPaths::new(&root)).unwrap();
    let router = app(AppState::new(AppPaths::new(&root)));
    let request = Request::builder()
        .method("PUT")
        .uri("/api/settings")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"providers":{"jellyfin":{"enabled":true,"url":"http://jellyfin.test"},"silo":{"enabled":true,"url":"http://silo.test"},"radarr":{"enabled":true,"url":"http://radarr.test"},"sonarr":{"enabled":true,"url":"http://sonarr.test"},"qbittorrent":{"enabled":true,"url":"http://qbit.test"}}}"#))
        .unwrap();
    assert_eq!(
        router.clone().oneshot(request).await.unwrap().status(),
        StatusCode::OK
    );
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/providers/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let statuses: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    for id in ["jellyfin", "silo", "radarr", "sonarr", "qbittorrent"] {
        let provider = statuses
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .unwrap();
        assert_eq!(provider["configured"], true, "{id}");
        assert_eq!(provider["enabled"], true, "{id}");
        assert_eq!(provider["online"], false, "{id}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn beszel_username_alias_is_private_email_and_disabling_preserves_it() {
    let root = std::env::temp_dir().join(format!("aooscope-beszel-{}", uuid::Uuid::new_v4()));
    aooscope_server::bootstrap(&AppPaths::new(&root)).unwrap();
    let router = app(AppState::new(AppPaths::new(&root)));
    let put = |body: &'static str| {
        router.clone().oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
    };
    assert_eq!(put(r#"{"providers":{"beszel":{"enabled":true,"url":"http://beszel.test","username":"user@example.test","password":"secret"}}}"#).await.unwrap().status(), StatusCode::OK);
    let public = get_json_from_router(router.clone(), "/api/settings").await;
    assert!(public["providers"]["beszel"].get("email").is_none());
    assert!(public["providers"]["beszel"].get("username").is_none());
    assert_eq!(public["providers"]["beszel"]["secret_set"], true);
    assert_eq!(
        put(r#"{"providers":{"beszel":{"enabled":false,"url":""}}}"#)
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let private: Value =
        serde_json::from_slice(&std::fs::read(root.join("private/providers.json")).unwrap())
            .unwrap();
    assert_eq!(private["beszel"]["email"], "user@example.test");
    assert_eq!(private["beszel"]["password"], "secret");
    assert!(private["beszel"].get("username").is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn qbittorrent_credentials_use_the_collector_secret_keys() {
    let root = std::env::temp_dir().join(format!("aooscope-qbittorrent-{}", uuid::Uuid::new_v4()));
    aooscope_server::bootstrap(&AppPaths::new(&root)).unwrap();
    let router = app(AppState::new(AppPaths::new(&root)));
    let response = router
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"providers":{"qbittorrent":{"enabled":true,"url":"http://qbit.test","username":"qbit-user","password":"qbit-password"}}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let private: Value =
        serde_json::from_slice(&std::fs::read(root.join("private/providers.json")).unwrap())
            .unwrap();
    assert_eq!(private["qbittorrent"]["username"], "qbit-user");
    assert_eq!(private["qbittorrent"]["password"], "qbit-password");
    assert!(private["qbittorrent"].get("email").is_none());
    let _ = std::fs::remove_dir_all(root);
}

async fn get_json_from_router(router: axum::Router, path: &str) -> Value {
    let response = router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
