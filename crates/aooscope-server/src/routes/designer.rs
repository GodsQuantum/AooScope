use aooscope_config::{ConfigError, atomic_write_json, load_pages, load_state};
use aooscope_render::{MediaError, MediaStore, RevisionStore, compile_document, compile_page};
use aooscope_types::{MediaAsset, MediaPreset, Page, PageBackground, validate_document};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::Response,
};
use image::ImageEncoder;
use serde_json::{Value, json};
use std::fs;

type RouteError = (StatusCode, Json<Value>);

fn error(status: StatusCode, message: &str) -> RouteError {
    (status, Json(json!({"ok":false,"error":message})))
}

fn config_error(_: ConfigError) -> RouteError {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "configuration unavailable",
    )
}

fn media_error(value: MediaError) -> RouteError {
    match value {
        MediaError::NotFound(_) => error(StatusCode::NOT_FOUND, "media asset not found"),
        MediaError::InUse(_) => error(StatusCode::CONFLICT, "media asset is in use"),
        MediaError::Invalid(_) => error(StatusCode::BAD_REQUEST, "invalid media"),
        MediaError::Io(_) | MediaError::Config(_) | MediaError::Image(_) => {
            error(StatusCode::INTERNAL_SERVER_ERROR, "media operation failed")
        }
    }
}

fn factory_page(template: &str, id: &str) -> Option<Page> {
    let (name, enabled, layers) = match template {
        "factory.splash.v1" => (
            "Splash",
            true,
            vec![
                json!({"id":"splash-brand","type":"text","x":80,"y":105,"width":800,"height":110,"z":2,"text":"AOOSCOPE"}),
                json!({"id":"splash-subtitle","type":"text","x":160,"y":235,"width":640,"height":44,"z":3,"text":"SMART LCD DASHBOARD"}),
            ],
        ),
        "factory.home.v1" | "home" => (
            "Home",
            true,
            vec![
                json!({"id":"home-cpu-gauge","type":"gauge","binding":"aooscope_pve_cpu_pct","x":40,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"home-cpu-value","type":"value","binding":"aooscope_pve_cpu_pct","x":80,"y":175,"width":160,"height":84,"z":5}),
                json!({"id":"home-ram-gauge","type":"gauge","binding":"aooscope_pve_memory_pct","x":360,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"home-ram-value","type":"value","binding":"aooscope_pve_memory_pct","x":400,"y":175,"width":160,"height":84,"z":5}),
                json!({"id":"home-temp-gauge","type":"gauge","binding":"aooscope_hardware_cpu_temp_c","x":680,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"home-temp-value","type":"value","binding":"aooscope_hardware_cpu_temp_c","x":720,"y":175,"width":160,"height":84,"z":5}),
            ],
        ),
        "factory.storage.v1" => (
            "Storage",
            true,
            vec![
                json!({"id":"storage-title","type":"text","x":28,"y":24,"width":420,"height":44,"z":5,"text":"STORAGE HEALTH"}),
                json!({"id":"storage-temp-0","type":"value","binding":"aooscope_pve_smart_0_temperature_c","x":40,"y":105,"width":120,"height":70,"z":4}),
                json!({"id":"storage-temp-1","type":"value","binding":"aooscope_pve_smart_1_temperature_c","x":350,"y":105,"width":120,"height":70,"z":4}),
                json!({"id":"storage-temp-2","type":"value","binding":"aooscope_pve_smart_2_temperature_c","x":660,"y":105,"width":120,"height":70,"z":4}),
                json!({"id":"storage-temp-3","type":"value","binding":"aooscope_pve_smart_3_temperature_c","x":40,"y":237,"width":120,"height":70,"z":4}),
                json!({"id":"storage-temp-4","type":"value","binding":"aooscope_pve_smart_4_temperature_c","x":350,"y":237,"width":120,"height":70,"z":4}),
                json!({"id":"storage-temp-5","type":"value","binding":"aooscope_pve_smart_5_temperature_c","x":660,"y":237,"width":120,"height":70,"z":4}),
                json!({"id":"storage-health-0","type":"badge","binding":"aooscope_pve_smart_0_health","x":165,"y":118,"width":112,"height":42,"z":3}),
                json!({"id":"storage-health-1","type":"badge","binding":"aooscope_pve_smart_1_health","x":475,"y":118,"width":112,"height":42,"z":3}),
                json!({"id":"storage-health-2","type":"badge","binding":"aooscope_pve_smart_2_health","x":785,"y":118,"width":112,"height":42,"z":3}),
                json!({"id":"storage-health-3","type":"badge","binding":"aooscope_pve_smart_3_health","x":165,"y":250,"width":112,"height":42,"z":3}),
                json!({"id":"storage-health-4","type":"badge","binding":"aooscope_pve_smart_4_health","x":475,"y":250,"width":112,"height":42,"z":3}),
                json!({"id":"storage-health-5","type":"badge","binding":"aooscope_pve_smart_5_health","x":785,"y":250,"width":112,"height":42,"z":3}),
            ],
        ),
        "factory.compute.v1" => (
            "Compute",
            true,
            vec![
                json!({"id":"compute-cpu-gauge","type":"gauge","binding":"aooscope_pve_cpu_pct","x":360,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"compute-gpu-gauge","type":"gauge","binding":"aooscope_hardware_gpu_busy_pct","x":40,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"compute-gpu-value","type":"value","binding":"aooscope_hardware_gpu_busy_pct","x":80,"y":175,"width":160,"height":84,"z":5}),
                json!({"id":"compute-cpu-value","type":"value","binding":"aooscope_pve_cpu_pct","x":400,"y":175,"width":160,"height":84,"z":5}),
                json!({"id":"compute-gtt-gauge","type":"gauge","binding":"aooscope_hardware_gpu_gtt_pct","x":680,"y":96,"width":240,"height":220,"z":1}),
                json!({"id":"compute-gtt-value","type":"value","binding":"aooscope_hardware_gpu_gtt_pct","x":720,"y":175,"width":160,"height":84,"z":5}),
            ],
        ),
        "factory.media.v1" => (
            "Media",
            false,
            vec![
                json!({"id":"media-headline","type":"text","binding":"aooscope_media_display_headline","x":320,"y":58,"width":590,"height":48,"z":6}),
                json!({"id":"media-title","type":"text","binding":"aooscope_media_display_title_short","x":320,"y":125,"width":590,"height":84,"z":6}),
                json!({"id":"media-progress","type":"bar","binding":"aooscope_media_display_progress_pct","x":320,"y":232,"width":570,"height":24,"z":2}),
                json!({"id":"media-progress-value","type":"value","binding":"aooscope_media_display_progress_pct","x":820,"y":266,"width":90,"height":38,"z":6}),
            ],
        ),
        _ => return None,
    };
    serde_json::from_value(json!({"id":id,"name":name,"enabled":enabled,"duration":8,"template_id":template,"revision":1,"background":{"color":"#071019"},"layers":layers})).ok()
}

async fn read_upload(form: &mut Multipart) -> Result<(String, Bytes), RouteError> {
    let field = form
        .next_field()
        .await
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid multipart upload"))?
        .ok_or_else(|| error(StatusCode::BAD_REQUEST, "missing file"))?;
    let name = field.file_name().unwrap_or("asset").to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid upload body"))?;
    Ok((name, bytes))
}

pub async fn get_page(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<Page>, RouteError> {
    let document = load_pages(&state.paths).map_err(config_error)?;
    document
        .pages
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "page not found"))
}

pub async fn create_page(
    State(state): State<crate::state::AppState>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Page>), RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    let id = format!("page-{}", uuid::Uuid::new_v4());
    let page = if let Some(template) = body.get("template_id").and_then(Value::as_str) {
        factory_page(template, &id)
            .ok_or_else(|| error(StatusCode::NOT_FOUND, "unknown template"))?
    } else {
        Page {
            id: id.clone(),
            name: body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("New page")
                .into(),
            enabled: true,
            duration: 8,
            template_id: None,
            revision: 1,
            background: PageBackground::default(),
            layers: Vec::new(),
            extra: Default::default(),
        }
    };
    document.carousel.push(id.clone());
    document.pages.insert(id, page.clone());
    validate_document(&document)
        .map_err(|issues| error(StatusCode::UNPROCESSABLE_ENTITY, &issues.join("; ")))?;
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok((StatusCode::CREATED, Json(page)))
}

pub async fn update_page(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Page>, RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    let old = document
        .pages
        .get(&id)
        .cloned()
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "page not found"))?;
    if body.get("revision").and_then(Value::as_u64) != Some(old.revision) {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"ok":false,"error":"revision conflict","current":old.revision})),
        ));
    }
    let mut candidate =
        serde_json::to_value(&old).map_err(|_| error(StatusCode::BAD_REQUEST, "invalid page"))?;
    if let (Some(target), Some(updates)) = (candidate.as_object_mut(), body.as_object()) {
        for (key, value) in updates {
            if key != "id" && key != "revision" {
                target.insert(key.clone(), value.clone());
            }
        }
    }
    candidate["id"] = json!(id);
    candidate["revision"] = json!(old.revision + 1);
    let page: Page = serde_json::from_value(candidate)
        .map_err(|_| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid page"))?;
    document.pages.insert(id, page.clone());
    validate_document(&document)
        .map_err(|issues| error(StatusCode::UNPROCESSABLE_ENTITY, &issues.join("; ")))?;
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok(Json(page))
}

pub async fn duplicate_page(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Page>), RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    let mut page = document
        .pages
        .get(&id)
        .cloned()
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "page not found"))?;
    page.id = format!("page-{}", uuid::Uuid::new_v4());
    page.name = format!("{} copy", page.name);
    page.revision = 1;
    document.carousel.push(page.id.clone());
    document.pages.insert(page.id.clone(), page.clone());
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok((StatusCode::CREATED, Json(page)))
}

pub async fn restore_page(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<Page>, RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    let old = document
        .pages
        .get(&id)
        .cloned()
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "page not found"))?;
    let template = old.template_id.as_deref().ok_or_else(|| {
        error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "page has no factory template",
        )
    })?;
    let mut page = factory_page(template, &id).ok_or_else(|| {
        error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "page has no factory template",
        )
    })?;
    page.revision = old.revision + 1;
    document.pages.insert(id.clone(), page.clone());
    validate_document(&document)
        .map_err(|issues| error(StatusCode::UNPROCESSABLE_ENTITY, &issues.join("; ")))?;
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok(Json(page))
}

pub async fn delete_page(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    if document.pages.len() <= 1 {
        return Err(error(StatusCode::CONFLICT, "last page"));
    }
    if document.pages.remove(&id).is_none() {
        return Err(error(StatusCode::NOT_FOUND, "page not found"));
    }
    document.carousel.retain(|item| item != &id);
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok(Json(json!({"ok":true,"revision":document.revision})))
}

pub async fn update_carousel(
    State(state): State<crate::state::AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, RouteError> {
    let mut document = load_pages(&state.paths).map_err(config_error)?;
    if body.get("revision").and_then(Value::as_u64) != Some(document.revision) {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"ok":false,"error":"revision conflict","current":document.revision})),
        ));
    }
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| error(StatusCode::UNPROCESSABLE_ENTITY, "invalid carousel"))?;
    let order: Vec<String> = items
        .iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    if order.len() != document.pages.len()
        || order.iter().collect::<std::collections::HashSet<_>>().len() != order.len()
        || order.iter().any(|id| !document.pages.contains_key(id))
    {
        return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "invalid carousel"));
    }
    for item in items {
        if let Some(page) = item
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| document.pages.get_mut(id))
        {
            if let Some(v) = item.get("enabled").and_then(Value::as_bool) {
                page.enabled = v;
            }
            if let Some(v) = item.get("duration").and_then(Value::as_u64) {
                page.duration = v as u32;
            }
        }
    }
    document.carousel = order;
    validate_document(&document)
        .map_err(|issues| error(StatusCode::UNPROCESSABLE_ENTITY, &issues.join("; ")))?;
    document.revision += 1;
    atomic_write_json(&state.paths.pages(), &document).map_err(config_error)?;
    Ok(Json(
        json!({"ok":true,"revision":document.revision,"carousel":document.carousel}),
    ))
}

pub async fn get_sensors(
    State(state): State<crate::state::AppState>,
) -> Result<Json<Value>, RouteError> {
    let state_value =
        serde_json::to_value(load_state(&state.paths).map_err(config_error)?).unwrap_or_default();
    let mut sensors = Vec::new();
    fn walk(value: &Value, path: &mut Vec<String>, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    path.push(key.clone());
                    walk(child, path, out);
                    path.pop();
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    path.push(index.to_string());
                    walk(child, path, out);
                    path.pop();
                }
            }
            Value::Null => {}
            leaf => {
                let key = format!("aooscope_{}", path.join("_"));
                out.push(json!({"key":key,"label":path.last().map(String::as_str).unwrap_or("value"),"group":path.first().map(String::as_str).unwrap_or("general"),"type":if leaf.is_number(){"number"}else{"string"},"value":leaf,"unit":""}));
            }
        }
    }
    walk(&state_value, &mut Vec::new(), &mut sensors);
    Ok(Json(json!({"sensors":sensors})))
}

pub async fn get_media(
    State(state): State<crate::state::AppState>,
) -> Result<Json<Value>, RouteError> {
    MediaStore::new(&state.paths.root)
        .and_then(|store| Ok(json!({"assets": store.list()?, "presets": store.presets()?})))
        .map(Json)
        .map_err(media_error)
}

pub async fn create_orbit_preset(
    State(state): State<crate::state::AppState>,
) -> Result<Json<MediaPreset>, RouteError> {
    let store = MediaStore::new(&state.paths.root).map_err(media_error)?;
    let preset = store.ensure_cloud9_orbit().map_err(media_error)?;
    let mut pages = load_pages(&state.paths).map_err(config_error)?;
    if aooscope_render::MediaStore::migrate_splash(&mut pages, &preset) {
        pages.revision += 1;
        atomic_write_json(&state.paths.pages(), &pages).map_err(config_error)?;
    }
    Ok(Json(preset))
}

pub async fn upload_media(
    State(state): State<crate::state::AppState>,
    mut form: Multipart,
) -> Result<(StatusCode, Json<MediaAsset>), RouteError> {
    let (name, bytes) = read_upload(&mut form).await?;
    MediaStore::new(&state.paths.root)
        .and_then(|store| store.ingest(&bytes, &name))
        .map(|asset| (StatusCode::CREATED, Json(asset)))
        .map_err(media_error)
}

pub async fn replace_media(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
    mut form: Multipart,
) -> Result<Json<MediaAsset>, RouteError> {
    let (name, bytes) = read_upload(&mut form).await?;
    MediaStore::new(&state.paths.root)
        .and_then(|store| store.replace(&id, &bytes, &name))
        .map(Json)
        .map_err(media_error)
}

pub async fn delete_media(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, RouteError> {
    let pages = load_pages(&state.paths).map_err(config_error)?;
    MediaStore::new(&state.paths.root)
        .and_then(|store| store.delete(&id, &pages))
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(media_error)
}

pub async fn media_file(
    State(state): State<crate::state::AppState>,
    Path(id): Path<String>,
) -> Result<Response, RouteError> {
    let store = MediaStore::new(&state.paths.root).map_err(media_error)?;
    let path = store.resolve(&id).map_err(media_error)?;
    let bytes =
        fs::read(&path).map_err(|_| error(StatusCode::NOT_FOUND, "media asset not found"))?;
    let content_type = mime_guess::from_path(&path).first_or_octet_stream();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type.as_ref())
        .body(Body::from(bytes))
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "media response failed"))
}

pub async fn preview(
    State(state): State<crate::state::AppState>,
    Json(body): Json<Value>,
) -> Result<Response, RouteError> {
    let page: Page = serde_json::from_value(body.get("page").cloned().unwrap_or(body))
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid page"))?;
    let media = MediaStore::new(&state.paths.root).map_err(media_error)?;
    let snapshot = load_state(&state.paths).map_err(config_error)?;
    let brightness = aooscope_config::load_settings(&state.paths)
        .map_err(config_error)?
        .display
        .brightness;
    let frame = compile_page(&page, &snapshot, &media, brightness, 0.0)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "page cannot be rendered"))?;
    let mut output = Vec::new();
    image::codecs::png::PngEncoder::new(&mut output)
        .write_image(
            frame.image.as_raw(),
            frame.image.width(),
            frame.image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "preview encoding failed"))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .body(Body::from(output))
        .expect("valid preview response"))
}

pub async fn apply(State(state): State<crate::state::AppState>) -> Result<Json<Value>, RouteError> {
    let pages = load_pages(&state.paths).map_err(config_error)?;
    let snapshot = load_state(&state.paths).map_err(config_error)?;
    let settings = aooscope_config::load_settings(&state.paths).map_err(config_error)?;
    let media = MediaStore::new(&state.paths.root).map_err(media_error)?;
    let compiled = compile_document(&pages, &snapshot, &media, settings.display.brightness, 0.0)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "pages cannot be rendered"))?;

    let store = RevisionStore::new(&state.paths.root);
    let revision_id = store
        .stage_compiled(
            serde_json::to_value(&pages)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "apply failed"))?,
            serde_json::to_value(&snapshot)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "apply failed"))?,
            &compiled,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "apply failed"))?;
    store
        .promote(&revision_id)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "apply failed"))?;
    state.promote_display(revision_id.clone());
    let warnings = compiled
        .pages
        .iter()
        .flat_map(|page| page.warnings.iter().cloned())
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"ok":true,"revision_id":revision_id,"warnings":warnings}),
    ))
}
