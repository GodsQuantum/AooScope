use aooscope_config::AppPaths;
use aooscope_display::SimulatedDisplayDriver;
use aooscope_server::{AppState, app, bootstrap};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::PathBuf};
use tower::ServiceExt;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aooscope-bootstrap-{name}-{}",
        uuid::Uuid::new_v4()
    ))
}

async fn status(app: &axum::Router, path: &str) -> StatusCode {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn empty_root_bootstraps_all_simulated_api_reads() {
    let root = root("api");
    let paths = AppPaths::new(&root);
    bootstrap(&paths).unwrap();
    let router = app(AppState::new(paths).with_display_driver(SimulatedDisplayDriver::new()));

    for path in [
        "/api/health",
        "/api/status",
        "/api/pages",
        "/api/metrics",
        "/api/media",
        "/api/display/capabilities",
        "/api/settings",
    ] {
        assert_eq!(status(&router, path).await, StatusCode::OK, "{path}");
    }
    let pages: Value = serde_json::from_slice(&fs::read(root.join("pages.json")).unwrap()).unwrap();
    assert_eq!(pages["schema_version"], 1);
    assert_eq!(pages["revision"], 1);
    assert_eq!(
        pages["carousel"],
        json!([
            "page-splash",
            "page-home",
            "page-storage",
            "page-compute",
            "page-media"
        ])
    );
    assert_eq!(pages["pages"]["page-splash"]["name"], "Splash");
    assert_eq!(pages["pages"]["page-home"]["name"], "Home");
    assert_eq!(pages["pages"]["page-storage"]["name"], "Storage");
    assert_eq!(pages["pages"]["page-compute"]["name"], "Compute");
    assert_eq!(pages["pages"]["page-media"]["name"], "Media");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(root.join("media.json")).unwrap()).unwrap()["assets"],
        json!({})
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(root.join("private/providers.json")).unwrap())
            .unwrap(),
        json!({})
    );
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(root.join("private/providers.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn bootstrap_is_idempotent_and_preserves_existing_documents() {
    let root = root("idempotent");
    let paths = AppPaths::new(&root);
    fs::create_dir_all(root.join("private")).unwrap();
    let existing = [
        (paths.settings(), br#"{"display":{"brightness":42},"unknown":true}"#.to_vec()),
        (paths.pages(), serde_json::to_vec(&json!({"schema_version":1,"revision":9,"carousel":["custom"],"pages":{"custom":{"id":"custom","name":"Custom","duration":8,"revision":1,"layers":[]}},"unknown":"keep"})).unwrap()),
        (paths.media(), br#"{"schema_version":1,"assets":{},"presets":{},"unknown":"keep"}"#.to_vec()),
        (paths.state(), br#"{"unknown":"keep"}"#.to_vec()),
        (paths.provider_secrets(), br#"{"custom":{"token":"keep"}}"#.to_vec()),
    ];
    for (path, bytes) in &existing {
        fs::write(path, bytes).unwrap();
    }
    #[cfg(unix)]
    let existing_mode = fs::metadata(paths.provider_secrets())
        .unwrap()
        .permissions()
        .mode();
    let before: Vec<_> = existing
        .iter()
        .map(|(path, _)| fs::read(path).unwrap())
        .collect();

    bootstrap(&paths).unwrap();
    bootstrap(&paths).unwrap();

    for ((path, _), expected) in existing.iter().zip(before) {
        assert_eq!(
            fs::read(path).unwrap(),
            expected,
            "{} changed",
            path.display()
        );
    }
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(paths.provider_secrets())
            .unwrap()
            .permissions()
            .mode(),
        existing_mode
    );
    let _ = fs::remove_dir_all(root);
}
