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
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;

type RouteError = (StatusCode, Json<Value>);

#[derive(Deserialize)]
pub struct OrbitPresetRequest {
    source_asset_id: String,
    display_name: Option<String>,
}

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

pub(crate) fn factory_page(template: &str, id: &str) -> Option<Page> {
    let (name, enabled, layers) = match template {
        "factory.splash.v1" => ("Splash", true, splash_layers()),
        "factory.home.v1" | "home" => ("Home", true, home_layers()),
        "factory.storage.v1" => ("Storage", true, storage_layers()),
        "factory.storage-m2.v1" => ("Storage M.2", false, storage_m2_layers()),
        "factory.compute.v1" => ("Compute", true, compute_layers()),
        "factory.media.v1" => ("Media", true, media_layers()),
        _ => return None,
    };
    serde_json::from_value(json!({"id":id,"name":name,"enabled":enabled,"duration":8,"template_id":template,"revision":1,"background":{"color":"#071019"},"layers":layers})).ok()
}

fn header(section: &str) -> Vec<Value> {
    vec![
        json!({"id":"header-brand","type":"text","x":28,"y":24,"width":260,"height":32,"z":8,"text":"AOOSCOPE","color":"#eaf7ff","scale":4}),
        json!({"id":"header-accent","type":"badge","x":28,"y":70,"width":120,"height":10,"z":8,"background_color":"#35d9ff","radius":5}),
        json!({"id":"header-section","type":"text","x":166,"y":68,"width":430,"height":20,"z":8,"text":section,"color":"#8fa2b7","scale":2}),
    ]
}

fn splash_layers() -> Vec<Value> {
    vec![
        json!({"id":"splash-halo","type":"ring","x":343,"y":38,"width":274,"height":274,"z":1,"color":"#35d9ff","track_color":"#172a3b","thickness":8,"value":72}),
        json!({"id":"splash-core","type":"badge","x":376,"y":71,"width":208,"height":208,"z":2,"background_color":"#0d1d2b","border_color":"#21445a","radius":104}),
        json!({"id":"splash-brand","type":"text","x":250,"y":158,"width":460,"height":64,"z":5,"text":"AOOSCOPE","color":"#eaf7ff","scale":8,"align":"center"}),
        json!({"id":"splash-subtitle","type":"text","x":300,"y":246,"width":360,"height":20,"z":5,"text":"SYSTEMS IN FOCUS","color":"#8fa2b7","scale":2,"align":"center"}),
    ]
}

fn gauge_group(
    id: &str,
    binding: &str,
    x: i32,
    color: &str,
    label: &str,
    unit: &str,
    range: (f64, f64),
) -> Vec<Value> {
    let (min, max) = range;
    vec![
        json!({"id":format!("{id}-gauge"),"type":"gauge","binding":binding,"x":x,"y":96,"width":220,"height":220,"z":1,"color":color,"track_color":"#1c2b3b","thickness":18,"min_value":min,"max_value":max}),
        json!({"id":format!("{id}-label"),"type":"text","x":x + 10,"y":116,"width":200,"height":24,"z":5,"text":label,"color":"#8fa2b7","scale":3,"align":"center"}),
        json!({"id":format!("{id}-value"),"type":"value","binding":binding,"x":x + 10,"y":179,"width":200,"height":64,"z":5,"color":"#f4fbff","unit":unit,"scale":7,"align":"center","fallback":"--"}),
    ]
}

fn home_layers() -> Vec<Value> {
    let mut layers = header("SYSTEM OVERVIEW");
    layers.extend(gauge_group(
        "home-cpu",
        "aooscope_pve_cpu_pct",
        45,
        "#35d9ff",
        "CPU",
        "%",
        (0.0, 100.0),
    ));
    layers.extend(gauge_group(
        "home-ram",
        "aooscope_pve_memory_pct",
        370,
        "#43d18b",
        "RAM",
        "%",
        (0.0, 100.0),
    ));
    layers.extend(gauge_group(
        "home-temp",
        "aooscope_hardware_cpu_temp_c",
        695,
        "#f2b84b",
        "CPU TEMP",
        " C",
        (20.0, 100.0),
    ));
    layers.extend([
        json!({"id":"home-cpu-caption","type":"text","x":90,"y":329,"width":130,"height":20,"z":5,"text":"LOAD","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-ram-caption","type":"text","x":405,"y":329,"width":150,"height":20,"z":5,"text":"MEMORY","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-temp-caption","type":"text","x":730,"y":329,"width":150,"height":20,"z":5,"text":"THERMAL","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-guests-label","type":"text","x":790,"y":26,"width":82,"height":16,"z":8,"text":"GUESTS","color":"#71859a","scale":2,"align":"right"}),
        json!({"id":"home-guests","type":"value","binding":"aooscope_pve_guests_running","x":886,"y":22,"width":46,"height":26,"z":8,"color":"#eaf7ff","scale":3,"align":"right","fallback":"0"}),
    ]);
    layers
}

fn storage_layers() -> Vec<Value> {
    let mut layers = header("STORAGE HEALTH");
    for index in 0..6 {
        let x = 28 + (index % 3) as i32 * 309;
        let y = 104 + (index / 3) as i32 * 132;
        layers.extend(storage_card("storage", "smart", "disks", index, x, y, 286));
    }
    layers
}

fn storage_card(
    prefix: &str,
    thermal: &str,
    devices: &str,
    index: usize,
    x: i32,
    y: i32,
    width: u32,
) -> Vec<Value> {
    let temp = format!("aooscope_pve_{thermal}_{index}_temperature_c");
    vec![
        json!({"id":format!("{prefix}-panel-{index}"),"type":"badge","x":x,"y":y,"width":width,"height":116,"z":0,"background_color":"#0d1824","border_color":"#263d50","radius":18}),
        json!({"id":format!("{prefix}-bar-{index}"),"type":"bar","binding":temp,"x":x + 15,"y":y + 18,"width":24,"height":80,"z":2,"color":"#35d9ff","track_color":"#1a2a39","orientation":"vertical","min_value":20,"max_value":70,"radius":12}),
        json!({"id":format!("{prefix}-name-{index}"),"type":"text","binding":format!("aooscope_pve_{devices}_{index}_name"),"x":x + 58,"y":y + 18,"width":width - 78,"height":20,"z":4,"color":"#dceaf3","scale":2}),
        json!({"id":format!("{prefix}-bay-{index}"),"type":"text","x":x + 58,"y":y + 45,"width":85,"height":16,"z":4,"text":format!("BAY {}", index + 1),"color":"#71859a","scale":2}),
        json!({"id":format!("{prefix}-temp-{index}"),"type":"value","binding":temp,"x":x + 58,"y":y + 70,"width":90,"height":30,"z":4,"color":"#f4fbff","unit":" C","scale":3,"fallback":"--"}),
        json!({"id":format!("{prefix}-health-{index}"),"type":"badge","binding":format!("aooscope_pve_{thermal}_{index}_health"),"x":x + width as i32 - 91,"y":y + 69,"width":72,"height":27,"z":4,"background_color":"#12382f","border_color":"#22614d","color":"#62e3a3","radius":13,"scale":2,"align":"center","valign":"center","fallback":"SMART"}),
    ]
}

fn storage_m2_layers() -> Vec<Value> {
    let mut layers = header("M.2 / NVME");
    for index in 0..4 {
        let x = 28 + (index % 2) * 465;
        let y = 103 + (index / 2) * 132;
        let temp = format!("aooscope_pve_nvme_{index}_temperature_c");
        layers.extend([
            json!({"id":format!("m2-panel-{index}"),"type":"badge","x":x,"y":y,"width":439,"height":116,"z":0,"background_color":"#0d1824","border_color":"#263d50","radius":18}),
            json!({"id":format!("m2-slot-{index}"),"type":"text","x":x + 20,"y":y + 18,"width":72,"height":20,"z":4,"text":format!("M.2 {}", index + 1),"color":"#8fa2b7","scale":2}),
            json!({"id":format!("m2-name-{index}"),"type":"text","binding":format!("aooscope_pve_nvme_{index}_name"),"x":x + 105,"y":y + 17,"width":235,"height":24,"z":4,"color":"#eaf7ff","scale":3}),
            json!({"id":format!("m2-role-{index}"),"type":"text","binding":format!("aooscope_pve_nvme_{index}_role"),"x":x + 20,"y":y + 51,"width":120,"height":18,"z":4,"color":"#71859a","scale":2}),
            json!({"id":format!("m2-bar-{index}"),"type":"bar","binding":temp,"x":x + 20,"y":y + 86,"width":280,"height":12,"z":3,"color":"#9a6cff","track_color":"#211d35","min_value":20,"max_value":90,"radius":6}),
            json!({"id":format!("m2-temp-{index}"),"type":"value","binding":temp,"x":x + 315,"y":y + 48,"width":100,"height":31,"z":4,"color":"#f4fbff","unit":" C","scale":3,"align":"right","fallback":"--"}),
            json!({"id":format!("m2-health-{index}"),"type":"badge","binding":format!("aooscope_pve_nvme_{index}_health"),"x":x + 315,"y":y + 82,"width":104,"height":24,"z":4,"background_color":"#12382f","color":"#62e3a3","radius":12,"scale":2,"align":"center","valign":"center","fallback":"HEALTHY"}),
        ]);
    }
    layers
}

fn compute_layers() -> Vec<Value> {
    let mut layers = header("COMPUTE");
    layers.extend(gauge_group(
        "compute-gpu",
        "aooscope_hardware_gpu_busy_pct",
        45,
        "#35d9ff",
        "GPU",
        "%",
        (0.0, 100.0),
    ));
    layers.extend(gauge_group(
        "compute-cpu",
        "aooscope_pve_cpu_pct",
        370,
        "#43d18b",
        "CPU",
        "%",
        (0.0, 100.0),
    ));
    layers.extend(gauge_group(
        "compute-gtt",
        "aooscope_hardware_gpu_gtt_pct",
        695,
        "#9a6cff",
        "SHARED",
        "%",
        (0.0, 100.0),
    ));
    layers.extend([
        json!({"id":"compute-gpu-temp-label","type":"text","x":72,"y":329,"width":110,"height":18,"z":5,"text":"GPU TEMP","color":"#71859a","scale":2}),
        json!({"id":"compute-gpu-temp","type":"value","binding":"aooscope_hardware_gpu_temp_c","x":188,"y":327,"width":74,"height":21,"z":5,"color":"#dceaf3","unit":" C","scale":2,"align":"right","fallback":"--"}),
        json!({"id":"compute-cpu-temp-label","type":"text","x":397,"y":329,"width":110,"height":18,"z":5,"text":"CPU TEMP","color":"#71859a","scale":2}),
        json!({"id":"compute-cpu-temp","type":"value","binding":"aooscope_hardware_cpu_temp_c","x":513,"y":327,"width":74,"height":21,"z":5,"color":"#dceaf3","unit":" C","scale":2,"align":"right","fallback":"--"}),
        json!({"id":"compute-gtt-caption","type":"text","x":747,"y":329,"width":116,"height":18,"z":5,"text":"UMA / GTT","color":"#71859a","scale":2,"align":"center"}),
    ]);
    layers
}

fn media_layers() -> Vec<Value> {
    vec![
        json!({"id":"media-poster-panel","type":"badge","x":22,"y":18,"width":264,"height":340,"z":0,"background_color":"#0d1b28","border_color":"#29445a","radius":22}),
        json!({"id":"media-poster-mark","type":"text","x":65,"y":148,"width":178,"height":45,"z":1,"text":"AOO","color":"#35d9ff","scale":6,"align":"center"}),
        json!({"id":"media-poster-brand","type":"text","x":49,"y":215,"width":212,"height":28,"z":1,"text":"AOOSCOPE","color":"#8fa2b7","scale":3,"align":"center"}),
        json!({"id":"media-poster","type":"image","binding":"aooscope_media_display_poster_asset_id","x":22,"y":18,"width":264,"height":340,"z":2,"optional":true}),
        json!({"id":"media-status-panel","type":"badge","x":306,"y":18,"width":632,"height":340,"z":0,"background_color":"#0d1b28","border_color":"#29445a","radius":22}),
        json!({"id":"media-accent","type":"badge","x":330,"y":42,"width":84,"height":8,"z":4,"background_color":"#35d9ff","radius":4}),
        json!({"id":"media-headline","type":"text","binding":"aooscope_media_display_headline","x":330,"y":68,"width":560,"height":27,"z":6,"color":"#62e3a3","scale":3}),
        json!({"id":"media-title","type":"text","binding":"aooscope_media_display_title_short","x":330,"y":116,"width":560,"height":72,"z":6,"color":"#f4fbff","scale":5}),
        json!({"id":"media-source-label","type":"text","x":330,"y":202,"width":92,"height":18,"z":6,"text":"SOURCE","color":"#71859a","scale":2}),
        json!({"id":"media-source","type":"text","binding":"aooscope_media_display_source","x":430,"y":201,"width":150,"height":20,"z":6,"color":"#b9cbd8","scale":2}),
        json!({"id":"media-provider-label","type":"text","x":600,"y":202,"width":110,"height":18,"z":6,"text":"PROVIDER","color":"#71859a","scale":2}),
        json!({"id":"media-provider","type":"text","binding":"aooscope_media_display_provider_chain","x":718,"y":201,"width":180,"height":20,"z":6,"color":"#b9cbd8","scale":2,"align":"right"}),
        json!({"id":"media-progress","type":"bar","binding":"aooscope_media_display_progress_pct","x":330,"y":242,"width":568,"height":18,"z":3,"color":"#35d9ff","track_color":"#1c2b3b","radius":9}),
        json!({"id":"media-progress-value","type":"value","binding":"aooscope_media_display_progress_pct","x":810,"y":275,"width":88,"height":24,"z":6,"color":"#eaf7ff","unit":"%","scale":3,"align":"right","fallback":"--"}),
        json!({"id":"media-eta-label","type":"text","x":330,"y":282,"width":60,"height":18,"z":6,"text":"ETA","color":"#71859a","scale":2}),
        json!({"id":"media-eta","type":"value","binding":"aooscope_media_display_eta_minutes","x":396,"y":279,"width":120,"height":24,"z":6,"color":"#dceaf3","unit":" MIN","scale":2,"fallback":"--"}),
        json!({"id":"media-rate-label","type":"text","x":330,"y":321,"width":60,"height":18,"z":6,"text":"RATE","color":"#71859a","scale":2}),
        json!({"id":"media-rate","type":"value","binding":"aooscope_media_display_speed_bytes_s","x":396,"y":318,"width":250,"height":24,"z":6,"color":"#dceaf3","format":"bytes_per_second","scale":2,"fallback":"--"}),
    ]
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
    Json(body): Json<OrbitPresetRequest>,
) -> Result<Json<MediaPreset>, RouteError> {
    let store = MediaStore::new(&state.paths.root).map_err(media_error)?;
    let preset = store
        .ensure_orbit(&body.source_asset_id, body.display_name.as_deref())
        .map_err(media_error)?;
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
