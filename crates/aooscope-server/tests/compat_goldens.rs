use aooscope_config::AppPaths;
use aooscope_render::{HEIGHT, MediaStore, WIDTH, compile_page};
use aooscope_server::{AppState, app};
use aooscope_types::{Page, StateDocument};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tower::ServiceExt;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/compat-v0.2")
}

fn goldens() -> Value {
    serde_json::from_slice(&fs::read(root().join("goldens.json")).unwrap()).unwrap()
}

fn normalized(path: &str, mut value: Value) -> Value {
    if matches!(path, "/api/health" | "/api/status") {
        value["version"] = json!("<version>");
    }
    value
}

async fn request(router: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn api_routes_match_compatibility_goldens() {
    let expected = goldens();
    let router = app(AppState::new(AppPaths::new(root())));
    for path in [
        "/api/health",
        "/api/status",
        "/api/settings",
        "/api/pages",
        "/api/media",
    ] {
        let (status, body) = request(
            &router,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;
        let golden = &expected["routes"][path];
        assert_eq!(status.as_u16(), golden["status"], "{path} status");
        if path == "/api/media" {
            assert_eq!(body["assets"], golden["json"]["assets"], "{path} assets");
            assert!(
                body["presets"].is_array(),
                "Rust media API keeps preset support"
            );
        } else {
            assert_eq!(
                normalized(path, body),
                normalized(path, golden["json"].clone()),
                "{path} body"
            );
        }
    }
}

#[tokio::test]
async fn factory_pages_match_compatibility_semantics() {
    let expected = goldens();
    let temp = std::env::temp_dir().join(format!("aooscope-compat-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    copy_dir(&root(), &temp);
    let router = app(AppState::new(AppPaths::new(&temp)));
    for template in [
        "factory.splash.v1",
        "factory.home.v1",
        "factory.storage.v1",
        "factory.compute.v1",
        "factory.media.v1",
    ] {
        let (_, page) = request(
            &router,
            Request::builder()
                .method("POST")
                .uri("/api/pages")
                .header("content-type", "application/json")
                .body(Body::from(json!({"template_id": template}).to_string()))
                .unwrap(),
        )
        .await;
        assert_eq!(
            semantic_page(&page),
            expected["factories"][template],
            "{template}"
        );
    }
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn renderer_and_panel_semantics_match_goldens() {
    let expected = goldens();
    let page: Page = serde_json::from_value(json!({
        "id":"factory.home.v1", "name":"Home", "enabled":true, "duration":8,
        "revision":1, "background":{"color":"#071019"},
        "layers": expected["factories"]["factory.home.v1"]["layers"]
    }))
    .unwrap();
    let state: StateDocument =
        serde_json::from_slice(&fs::read(root().join("state.json")).unwrap()).unwrap();
    let media = MediaStore::new(root()).unwrap();
    let compiled = compile_page(&page, &state, &media, 100, 0.0).unwrap();
    assert_eq!(
        [compiled.image.width(), compiled.image.height()],
        [WIDTH, HEIGHT]
    );
    assert_eq!(
        serde_json::to_value(compiled.warnings).unwrap(),
        expected["renderers"]["compile"]["warnings"]
    );
    assert_eq!(expected["renderers"]["idle"], "MEDIA READY");
    assert_eq!(expected["renderers"]["incoming"], "READY IN 12 MIN");
    assert_eq!(expected["renderers"]["playing"], "PLAYING 38%");
    assert_eq!(expected["renderers"]["landed"], "JUST LANDED");
    let panel_ids: Vec<&str> = expected["panels"]["normal"]["diy"]
        .as_array()
        .unwrap()
        .iter()
        .map(|panel| panel["id"].as_str().unwrap())
        .collect();
    assert_eq!(panel_ids, ["home", "storage", "compute"]);
}

fn semantic_page(page: &Value) -> Value {
    let mut result = json!({
        "name": page["name"], "enabled": page["enabled"],
        "duration": page["duration"], "background": page["background"],
        "layers": [],
    });
    result["layers"] = Value::Array(page["layers"].as_array().unwrap().iter().map(|layer| {
        let mut out = json!({
            "id": layer["id"], "type": layer["type"], "x": layer["x"],
            "y": layer["y"], "width": layer["width"], "height": layer["height"], "z": layer["z"],
        });
        if let Some(binding) = layer.get("binding") { out["binding"] = binding.clone(); }
        out
    }).collect());
    result
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
