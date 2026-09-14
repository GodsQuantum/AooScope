use aooscope_config::AppPaths;
use aooscope_render::MediaStore;
use aooscope_server::{AppState, app, bootstrap};
use aooscope_types::PagesDocument;
use axum::{body::Body, http::Request};
use serde_json::{Value, json};
use std::{fs, path::Path};
use tower::ServiceExt;

#[tokio::test]
async fn designer_routes_read_fixture_and_apply_revision() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1");
    let temp = std::env::temp_dir().join(format!("aooscope-designer-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    for name in ["pages.json", "media.json", "state.json", "settings.json"] {
        fs::copy(source.join(name), temp.join(name)).unwrap();
    }
    let router = app(AppState::new(AppPaths::new(&temp)));
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/media")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/apply")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn task_three_compatibility_contracts() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1");
    let temp = std::env::temp_dir().join(format!("aooscope-task3-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    for name in ["pages.json", "media.json", "state.json", "settings.json"] {
        fs::copy(source.join(name), temp.join(name)).unwrap();
    }
    let router = app(AppState::new(AppPaths::new(&temp)));

    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/api/pages",
            json!({"template_id":"factory.home.v1"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    let created: Value = body(response).await;
    let created_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["template_id"], "factory.home.v1");
    assert!(!created["layers"].as_array().unwrap().is_empty());

    let page_response = router
        .clone()
        .oneshot(request("GET", "/api/pages/page-home", Value::Null))
        .await
        .unwrap();
    let page: Value = body(page_response).await;
    let revision = page["revision"].as_u64().unwrap();
    let response = router
        .clone()
        .oneshot(request(
            "PUT",
            "/api/pages/page-home",
            json!({"revision":revision,"name":"Renamed"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let updated: Value = body(response).await;
    assert_eq!(updated["name"], "Renamed");
    assert!(updated["layers"].as_array().is_some());
    let response = router
        .clone()
        .oneshot(request(
            "PUT",
            "/api/pages/page-home",
            json!({"revision":revision,"name":"stale"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 409);

    let response = router
        .clone()
        .oneshot(request("POST", "/api/pages/page-home/restore", Value::Null))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let restored: Value = body(response).await;
    assert_eq!(restored["id"], "page-home");
    assert_eq!(restored["template_id"], "home");
    assert_eq!(restored["revision"], revision + 2);

    let pages: Value = body(
        router
            .clone()
            .oneshot(request("GET", "/api/pages", Value::Null))
            .await
            .unwrap(),
    )
    .await;
    let items = pages["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({"id":p["id"],"enabled":p["enabled"],"duration":p["duration"]}))
        .collect::<Vec<_>>();
    let response = router
        .clone()
        .oneshot(request(
            "PUT",
            "/api/carousel",
            json!({"revision":pages["revision"],"items":items}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(
        body(
            router
                .clone()
                .oneshot(request("GET", "/api/sensors", Value::Null))
                .await
                .unwrap()
        )
        .await["sensors"]
            .is_array()
    );

    let png = image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(png)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let asset = MediaStore::new(&temp)
        .unwrap()
        .ingest(&bytes, "test.png")
        .unwrap();
    let response = router
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/media/{}/file", asset.id),
            Value::Null,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    let response = router
        .clone()
        .oneshot(request("POST", "/api/preview", json!({"page":restored})))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");
    assert!(
        !axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let response = router
        .clone()
        .oneshot(request("POST", "/api/apply", Value::Null))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let applied: Value = body(response).await;
    let revision_dir = temp
        .join("compiled")
        .join(applied["revision_id"].as_str().unwrap());
    assert!(revision_dir.join("frame-0.png").is_file());
    assert!(revision_dir.join("manifest.json").is_file());

    let response = router
        .oneshot(request(
            "DELETE",
            &format!("/api/pages/{created_id}"),
            Value::Null,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn invalid_page_update_is_rejected_without_changing_pages_json() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1");
    let temp = std::env::temp_dir().join(format!("aooscope-invalid-page-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    for name in ["pages.json", "media.json", "state.json", "settings.json"] {
        fs::copy(source.join(name), temp.join(name)).unwrap();
    }
    let router = app(AppState::new(AppPaths::new(&temp)));
    let before = fs::read(temp.join("pages.json")).unwrap();
    let page: Value = body(
        router
            .clone()
            .oneshot(request("GET", "/api/pages/page-home", Value::Null))
            .await
            .unwrap(),
    )
    .await;
    let response = router
        .oneshot(request(
            "PUT",
            "/api/pages/page-home",
            json!({"revision":page["revision"],"duration":121}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 422);
    assert_eq!(before, fs::read(temp.join("pages.json")).unwrap());
}

#[tokio::test]
async fn orbit_preset_accepts_explicit_source_and_updates_the_same_preset() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1");
    let temp = std::env::temp_dir().join(format!("aooscope-orbit-api-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();
    for name in ["pages.json", "media.json", "state.json", "settings.json"] {
        fs::copy(source.join(name), temp.join(name)).unwrap();
    }
    let router = app(AppState::new(AppPaths::new(&temp)));
    let response = router
        .clone()
        .oneshot(request(
            "POST",
            "/api/media/presets/orbit",
            json!({"source_asset_id":"fixture-image","display_name":"Logo Orbit"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let preset = body(response).await;
    assert_eq!(preset["id"], "orbit");
    assert_eq!(preset["name"], "Logo Orbit");
    assert_eq!(preset["settings"]["fps"], 5);
    assert_eq!(preset["settings"]["speed_seconds"], 4);

    let response = router
        .oneshot(request(
            "POST",
            "/api/media/presets/orbit",
            json!({"source_asset_id":"fixture-image"}),
        ))
        .await
        .unwrap();
    assert_eq!(body(response).await["name"], "Orbit");
}

#[tokio::test]
async fn orbit_preset_updates_only_the_bootstrapped_factory_splash() {
    let temp =
        std::env::temp_dir().join(format!("aooscope-orbit-bootstrap-{}", uuid::Uuid::new_v4()));
    let paths = AppPaths::new(&temp);
    bootstrap(&paths).unwrap();

    let png = image::RgbImage::from_pixel(2, 2, image::Rgb([9, 8, 7]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(png)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let asset = MediaStore::new(&temp)
        .unwrap()
        .ingest(&bytes, "orbit.png")
        .unwrap();

    let mut pages: PagesDocument =
        serde_json::from_slice(&fs::read(paths.pages()).unwrap()).unwrap();
    pages.pages.insert(
        "custom-splash".into(),
        serde_json::from_value(json!({
            "id":"custom-splash", "name":"Splash", "duration":8, "revision":7,
            "layers":[{"id":"custom-logo","type":"image","asset_id":asset.id,"x":0,"y":0,"width":2,"height":2,"z":1}]
        }))
        .unwrap(),
    );
    fs::write(paths.pages(), serde_json::to_vec_pretty(&pages).unwrap()).unwrap();
    let custom_before = serde_json::to_vec(&pages.pages["custom-splash"]).unwrap();

    let response = app(AppState::new(paths.clone()))
        .oneshot(request(
            "POST",
            "/api/media/presets/orbit",
            json!({"source_asset_id":asset.id}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let pages: PagesDocument = serde_json::from_slice(&fs::read(paths.pages()).unwrap()).unwrap();
    let splash = &pages.pages["page-splash"];
    assert_eq!(splash.template_id.as_deref(), Some("factory.splash.v1"));
    let orbit = splash
        .layers
        .iter()
        .find(|layer| layer.layer_type == "animation")
        .unwrap();
    assert_eq!(orbit.extra["asset_id"], asset.id);
    assert_eq!(orbit.extra["preset_id"], "orbit");
    assert_eq!(
        serde_json::to_vec(&pages.pages["custom-splash"]).unwrap(),
        custom_before
    );
    let _ = fs::remove_dir_all(temp);
}

fn request(method: &str, uri: &str, value: Value) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if !value.is_null() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(if value.is_null() {
            Body::empty()
        } else {
            Body::from(value.to_string())
        })
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
