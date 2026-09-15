use crate::storage::StorageDevice;
use aooscope_types::{Layer, Page, PageBackground};
use serde_json::{Value, json};

const CARD_LIMIT: usize = 6;

pub fn factory_page(template: &str, id: &str) -> Option<Page> {
    let (name, enabled, layers) = match template {
        "factory.splash.v1" => ("Splash", true, splash_layers()),
        "factory.home.v1" | "home" => ("Home", true, home_layers()),
        "factory.storage.v1" => ("Storage", true, storage_layers()),
        "factory.storage-m2.v1" => ("Storage M.2", false, storage_m2_layers()),
        "factory.compute.v1" => ("Compute", true, compute_layers()),
        "factory.media.v1" => ("Media", true, media_layers()),
        "factory.vertical-bars.v1" => ("Vertical bars", true, vertical_bars_layers()),
        "factory.semi-rings.v1" => ("Semi rings", true, semi_rings_layers()),
        "factory.horizontal-bars.v1" => ("Horizontal bars", true, horizontal_bars_layers()),
        _ => return None,
    };
    Some(page(id, name, enabled, template, layers))
}

pub fn storage_pages(devices: &[StorageDevice]) -> Vec<Page> {
    if devices.is_empty() {
        return Vec::new();
    }
    let page_count = devices.len().div_ceil(CARD_LIMIT);
    let page_size = devices.len().div_ceil(page_count);
    devices
        .chunks(page_size)
        .enumerate()
        .map(|(page_index, chunk)| {
            let id = if page_index == 0 {
                "page-storage".to_owned()
            } else {
                format!("page-storage-{}", page_index + 1)
            };
            let columns = if chunk.len() <= 4 { 2 } else { 3 };
            let width = if columns == 2 { 439 } else { 286 };
            let mut layers = header("STORAGE HEALTH");
            for (index, device) in chunk.iter().enumerate() {
                let bounds = (
                    28 + (index % columns) as i32 * if columns == 2 { 465 } else { 309 },
                    104 + (index / columns) as i32 * 132,
                    width,
                    116,
                );
                layers.extend(storage_card_layers(device, bounds));
            }
            page(
                &id,
                "Storage",
                !chunk.is_empty(),
                "factory.storage.v1",
                layers,
            )
        })
        .collect()
}

pub fn storage_card_layers(
    device: &StorageDevice,
    (x, y, width, height): (i32, i32, u32, u32),
) -> Vec<Layer> {
    let index = device.index;
    let usage = format!("aooscope_pve_disks_{index}_usage_pct");
    let temperature = format!("aooscope_pve_smart_{index}_temperature_c");
    let capacity_value_width = (width.saturating_sub(143)) / 2;
    let separator_x = x + 105 + capacity_value_width as i32;
    let total_x = separator_x + 18;
    let layers = vec![
        json!({"id":format!("storage-panel-{index}"),"type":"badge","x":x,"y":y,"width":width,"height":height,"z":0,"background_color":"#0d1824","border_color":"#263d50","radius":18}),
        json!({"id":format!("storage-name-{index}"),"type":"text","binding":format!("aooscope_pve_disks_{index}_name"),"x":x + 20,"y":y + 16,"width":width - 40,"height":22,"z":4,"color":"#dceaf3","scale":2}),
        json!({"id":format!("storage-capacity-{index}"),"type":"text","x":x + 20,"y":y + 43,"width":width - 40,"height":18,"z":4,"text":"CAPACITY","color":"#71859a","scale":2}),
        json!({"id":format!("storage-used-{index}"),"type":"value","binding":format!("aooscope_pve_disks_{index}_used_bytes"),"x":x + 105,"y":y + 39,"width":capacity_value_width,"height":24,"z":4,"color":"#f4fbff","scale":2,"align":"right","fallback":"--","format":"bytes"}),
        json!({"id":format!("storage-total-{index}"),"type":"value","binding":format!("aooscope_pve_disks_{index}_size_bytes"),"x":total_x,"y":y + 39,"width":capacity_value_width,"height":24,"z":4,"color":"#b9cbd8","scale":2,"align":"right","fallback":"--","format":"bytes"}),
        json!({"id":format!("storage-separator-{index}"),"type":"text","x":separator_x,"y":y + 39,"width":18,"height":24,"z":5,"text":"/","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":format!("storage-bar-{index}"),"type":"bar","binding":usage,"x":x + 20,"y":y + 70,"width":width - 40,"height":8,"z":3,"color":"#35d9ff","track_color":"#1a2a39","min_value":0,"max_value":100,"radius":4}),
        json!({"id":format!("storage-temp-{index}"),"type":"value","binding":temperature,"x":x + 20,"y":y + 88,"width":90,"height":20,"z":4,"color":"#f4fbff","unit":" °C","scale":2,"fallback":"--"}),
        json!({"id":format!("storage-usage-{index}"),"type":"value","binding":usage,"x":x + width as i32 / 2 - 35,"y":y + 88,"width":70,"height":20,"z":4,"color":"#9fb3c2","unit":"%","scale":2,"align":"center","fallback":"--"}),
        json!({"id":format!("storage-health-{index}"),"type":"badge","binding":format!("aooscope_pve_smart_{index}_health"),"x":x + width as i32 - 101,"y":y + 86,"width":81,"height":24,"z":4,"background_color":"#12382f","border_color":"#22614d","color":"#62e3a3","radius":12,"scale":2,"align":"center","valign":"center","fallback":"SMART"}),
    ];
    layers
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .collect()
}

fn page(id: &str, name: &str, enabled: bool, template: &str, layers: Vec<Layer>) -> Page {
    Page {
        id: id.into(),
        name: name.into(),
        enabled,
        duration: 8,
        template_id: Some(template.into()),
        revision: 1,
        background: PageBackground {
            color: "#071019".into(),
            extra: Default::default(),
        },
        layers,
        extra: Default::default(),
    }
}

fn layer(value: Value) -> Layer {
    serde_json::from_value(value).unwrap()
}

fn layers(values: Vec<Value>) -> Vec<Layer> {
    values.into_iter().map(layer).collect()
}

fn header(section: &str) -> Vec<Layer> {
    layers(vec![
        json!({"id":"header-brand","type":"text","x":28,"y":24,"width":260,"height":32,"z":8,"text":"AOOSCOPE","color":"#eaf7ff","scale":4}),
        json!({"id":"header-accent","type":"badge","x":28,"y":70,"width":120,"height":10,"z":8,"background_color":"#35d9ff","radius":5}),
        json!({"id":"header-section","type":"text","x":166,"y":68,"width":430,"height":20,"z":8,"text":section,"color":"#8fa2b7","scale":2}),
    ])
}

fn gauge_group(
    id: &str,
    binding: &str,
    x: i32,
    color: &str,
    label: &str,
    unit: &str,
    range: (f64, f64),
) -> Vec<Layer> {
    let (min, max) = range;
    layers(vec![
        json!({"id":format!("{id}-gauge"),"type":"gauge","binding":binding,"x":x,"y":96,"width":220,"height":220,"z":1,"color":color,"track_color":"#1c2b3b","thickness":18,"min_value":min,"max_value":max}),
        json!({"id":format!("{id}-label"),"type":"text","x":x + 10,"y":116,"width":200,"height":24,"z":5,"text":label,"color":"#8fa2b7","scale":3,"align":"center"}),
        json!({"id":format!("{id}-value"),"type":"value","binding":binding,"x":x + 10,"y":179,"width":200,"height":64,"z":5,"color":"#f4fbff","unit":unit,"scale":7,"align":"center","fallback":"--"}),
    ])
}

fn splash_layers() -> Vec<Layer> {
    layers(vec![
        json!({"id":"splash-halo","type":"ring","x":343,"y":38,"width":274,"height":274,"z":1,"color":"#35d9ff","track_color":"#172a3b","thickness":8,"value":72}),
        json!({"id":"splash-core","type":"badge","x":376,"y":71,"width":208,"height":208,"z":2,"background_color":"#0d1d2b","border_color":"#21445a","radius":104}),
        json!({"id":"splash-brand","type":"text","x":250,"y":158,"width":460,"height":64,"z":5,"text":"AOOSCOPE","color":"#eaf7ff","scale":8,"align":"center"}),
        json!({"id":"splash-subtitle","type":"text","x":300,"y":246,"width":360,"height":20,"z":5,"text":"SYSTEMS IN FOCUS","color":"#8fa2b7","scale":2,"align":"center"}),
    ])
}

fn home_layers() -> Vec<Layer> {
    let mut result = header("SYSTEM OVERVIEW");
    result.extend(gauge_group(
        "home-cpu",
        "aooscope_pve_cpu_pct",
        45,
        "#35d9ff",
        "CPU",
        "%",
        (0.0, 100.0),
    ));
    result.extend(gauge_group(
        "home-ram",
        "aooscope_pve_memory_pct",
        370,
        "#43d18b",
        "RAM",
        "%",
        (0.0, 100.0),
    ));
    result.extend(gauge_group(
        "home-temp",
        "aooscope_hardware_cpu_temp_c",
        695,
        "#f2b84b",
        "CPU TEMP",
        " C",
        (20.0, 100.0),
    ));
    result.extend(layers(vec![
        json!({"id":"home-cpu-caption","type":"text","x":90,"y":329,"width":130,"height":20,"z":5,"text":"LOAD","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-ram-caption","type":"text","x":405,"y":329,"width":150,"height":20,"z":5,"text":"MEMORY","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-temp-caption","type":"text","x":730,"y":329,"width":150,"height":20,"z":5,"text":"THERMAL","color":"#71859a","scale":2,"align":"center"}),
        json!({"id":"home-guests-label","type":"text","x":790,"y":26,"width":82,"height":16,"z":8,"text":"GUESTS","color":"#71859a","scale":2,"align":"right"}),
        json!({"id":"home-guests","type":"value","binding":"aooscope_pve_guests_running","x":886,"y":22,"width":46,"height":26,"z":8,"color":"#eaf7ff","scale":3,"align":"right","fallback":"0"}),
    ]));
    result
}

fn storage_layers() -> Vec<Layer> {
    let devices = (0..6)
        .map(|index| StorageDevice {
            index,
            path: String::new(),
            label: String::new(),
            kind: String::new(),
            total_bytes: 0,
            used_bytes: None,
            free_bytes: None,
            usage_pct: None,
            temperature_c: None,
            health: None,
        })
        .collect::<Vec<_>>();
    let mut result = header("STORAGE HEALTH");
    for (index, device) in devices.iter().enumerate() {
        result.extend(storage_card_layers(
            device,
            (
                28 + (index % 3) as i32 * 309,
                104 + (index / 3) as i32 * 132,
                286,
                116,
            ),
        ));
    }
    result
}

fn storage_m2_layers() -> Vec<Layer> {
    let mut result = header("M.2 / NVME");
    for index in 0..4 {
        let x = 28 + (index % 2) * 465;
        let y = 103 + (index / 2) * 132;
        let temperature = format!("aooscope_pve_nvme_{index}_temperature_c");
        result.extend(layers(vec![
            json!({"id":format!("m2-panel-{index}"),"type":"badge","x":x,"y":y,"width":439,"height":116,"z":0,"background_color":"#0d1824","border_color":"#263d50","radius":18}),
            json!({"id":format!("m2-slot-{index}"),"type":"text","x":x + 20,"y":y + 18,"width":72,"height":20,"z":4,"text":format!("M.2 {}", index + 1),"color":"#8fa2b7","scale":2}),
            json!({"id":format!("m2-name-{index}"),"type":"text","binding":format!("aooscope_pve_nvme_{index}_name"),"x":x + 105,"y":y + 17,"width":235,"height":24,"z":4,"color":"#eaf7ff","scale":3}),
            json!({"id":format!("m2-role-{index}"),"type":"text","binding":format!("aooscope_pve_nvme_{index}_role"),"x":x + 20,"y":y + 51,"width":120,"height":18,"z":4,"color":"#71859a","scale":2}),
            json!({"id":format!("m2-bar-{index}"),"type":"bar","binding":temperature,"x":x + 20,"y":y + 86,"width":280,"height":12,"z":3,"color":"#9a6cff","track_color":"#211d35","min_value":20,"max_value":90,"radius":6}),
            json!({"id":format!("m2-temp-{index}"),"type":"value","binding":temperature,"x":x + 315,"y":y + 48,"width":100,"height":31,"z":4,"color":"#f4fbff","unit":" C","scale":3,"align":"right","fallback":"--"}),
            json!({"id":format!("m2-health-{index}"),"type":"badge","binding":format!("aooscope_pve_nvme_{index}_health"),"x":x + 315,"y":y + 82,"width":104,"height":24,"z":4,"background_color":"#12382f","color":"#62e3a3","radius":12,"scale":2,"align":"center","valign":"center","fallback":"HEALTHY"}),
        ]));
    }
    result
}

fn compute_layers() -> Vec<Layer> {
    let mut result = header("COMPUTE");
    result.extend(gauge_group(
        "compute-gpu",
        "aooscope_hardware_gpu_busy_pct",
        45,
        "#35d9ff",
        "GPU",
        "%",
        (0.0, 100.0),
    ));
    result.extend(gauge_group(
        "compute-cpu",
        "aooscope_pve_cpu_pct",
        370,
        "#43d18b",
        "CPU",
        "%",
        (0.0, 100.0),
    ));
    result.extend(gauge_group(
        "compute-gtt",
        "aooscope_hardware_gpu_gtt_pct",
        695,
        "#9a6cff",
        "SHARED",
        "%",
        (0.0, 100.0),
    ));
    result.extend(layers(vec![
        json!({"id":"compute-gpu-temp-label","type":"text","x":72,"y":329,"width":110,"height":18,"z":5,"text":"GPU TEMP","color":"#71859a","scale":2}),
        json!({"id":"compute-gpu-temp","type":"value","binding":"aooscope_hardware_gpu_temp_c","x":188,"y":327,"width":74,"height":21,"z":5,"color":"#dceaf3","unit":" C","scale":2,"align":"right","fallback":"--"}),
        json!({"id":"compute-cpu-temp-label","type":"text","x":397,"y":329,"width":110,"height":18,"z":5,"text":"CPU TEMP","color":"#71859a","scale":2}),
        json!({"id":"compute-cpu-temp","type":"value","binding":"aooscope_hardware_cpu_temp_c","x":513,"y":327,"width":74,"height":21,"z":5,"color":"#dceaf3","unit":" C","scale":2,"align":"right","fallback":"--"}),
        json!({"id":"compute-gtt-caption","type":"text","x":747,"y":329,"width":116,"height":18,"z":5,"text":"UMA / GTT","color":"#71859a","scale":2,"align":"center"}),
    ]));
    result
}

fn media_layers() -> Vec<Layer> {
    layers(vec![
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
    ])
}

fn vertical_bars_layers() -> Vec<Layer> {
    let mut result = header("VERTICAL BARS");
    for (index, (binding, label, color)) in [
        ("aooscope_pve_cpu_pct", "CPU", "#35d9ff"),
        ("aooscope_pve_memory_pct", "RAM", "#43d18b"),
        ("aooscope_hardware_gpu_busy_pct", "GPU", "#9a6cff"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 160 + index as i32 * 220;
        result.extend(layers(vec![
            json!({"id":format!("vertical-bar-{index}"),"type":"bar","binding":binding,"x":x,"y":110,"width":56,"height":190,"z":1,"color":color,"track_color":"#1a2a39","orientation":"vertical","min_value":0,"max_value":100,"radius":12}),
            json!({"id":format!("vertical-label-{index}"),"type":"text","x":x - 40,"y":316,"width":136,"height":24,"z":2,"text":label,"color":"#dceaf3","scale":3,"align":"center"}),
        ]));
    }
    result
}

fn semi_rings_layers() -> Vec<Layer> {
    let mut result = header("SEMI RINGS");
    for (index, (binding, label, color)) in [
        ("aooscope_pve_cpu_pct", "CPU", "#35d9ff"),
        ("aooscope_pve_memory_pct", "RAM", "#43d18b"),
        ("aooscope_hardware_gpu_busy_pct", "GPU", "#9a6cff"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 70 + index as i32 * 285;
        result.extend(gauge_group(
            &format!("semi-ring-{index}"),
            binding,
            x,
            color,
            label,
            "%",
            (0.0, 100.0),
        ));
    }
    result
}

fn horizontal_bars_layers() -> Vec<Layer> {
    let mut result = header("HORIZONTAL BARS");
    for (index, (binding, label, color)) in [
        ("aooscope_pve_cpu_pct", "CPU", "#35d9ff"),
        ("aooscope_pve_memory_pct", "RAM", "#43d18b"),
        ("aooscope_hardware_gpu_busy_pct", "GPU", "#9a6cff"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = 112 + index as i32 * 70;
        result.extend(layers(vec![
            json!({"id":format!("horizontal-label-{index}"),"type":"text","x":80,"y":y,"width":90,"height":24,"z":2,"text":label,"color":"#dceaf3","scale":3}),
            json!({"id":format!("horizontal-bar-{index}"),"type":"bar","binding":binding,"x":190,"y":y + 5,"width":680,"height":12,"z":1,"color":color,"track_color":"#1a2a39","min_value":0,"max_value":100,"radius":6}),
        ]));
    }
    result
}
