use aooscope_render::{HEIGHT, WIDTH, compile_document, compile_page};
use aooscope_types::{Page, PagesDocument, StateDocument};
use serde_json::json;
use std::path::PathBuf;

fn page(kind: &str) -> Page {
    serde_json::from_value(json!({"id":"home","name":"Home","enabled":true,"duration":8,"revision":1,"background":{"color":"#071019"},"layers":[{"id":"w","type":kind,"binding":"aooscope_pve_cpu_pct","x":10,"y":10,"width":200,"height":80,"z":1,"color":"#35d9ff"}]})).unwrap()
}

#[test]
fn every_widget_compiles_to_exact_rgb_canvas() {
    let root = temp_root("compiler");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    for kind in [
        "text",
        "value",
        "bar",
        "gauge",
        "ring",
        "badge",
        "sparkline",
        "image",
        "animation",
    ] {
        let mut p = page(kind);
        if kind == "text" {
            p.extra.insert("text".into(), json!("Home"));
        }
        let frame = compile_page(&p, &StateDocument::default(), &media, 100, 0.0).unwrap();
        assert_eq!((frame.image.width(), frame.image.height()), (WIDTH, HEIGHT));
        assert_eq!(frame.image.as_raw().len(), (WIDTH * HEIGHT * 3) as usize);
    }
}

#[test]
fn document_keeps_enabled_carousel_order_and_schedule() {
    let root = temp_root("document");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let doc: PagesDocument = serde_json::from_value(json!({"schema_version":1,"revision":3,"carousel":["home","storage"],"pages":{"home":serde_json::to_value(page("value")).unwrap(),"storage":json!({"id":"storage","name":"Storage","enabled":false,"duration":8,"revision":1,"layers":[]})}})).unwrap();
    let result = compile_document(&doc, &StateDocument::default(), &media, 100, 0.0).unwrap();
    assert_eq!(result.pages.len(), 1);
    assert_eq!(result.switch_seconds, 8);
}

#[test]
fn text_and_bound_values_are_rasterized() {
    let root = temp_root("text");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let mut static_page = page("text");
    static_page.layers[0].binding = None;
    static_page.layers[0]
        .extra
        .insert("text".into(), json!("STATIC TEXT"));
    let static_frame =
        compile_page(&static_page, &StateDocument::default(), &media, 100, 0.0).unwrap();
    assert!(
        static_frame
            .image
            .as_raw()
            .chunks(3)
            .filter(|p| p != &[7, 16, 25])
            .count()
            > 100
    );
    let changed = StateDocument {
        pve: Some(json!({"cpu_pct": 91})),
        ..Default::default()
    };
    let value_frame = compile_page(&page("value"), &changed, &media, 100, 0.0).unwrap();
    let empty_frame =
        compile_page(&page("value"), &StateDocument::default(), &media, 100, 0.0).unwrap();
    assert_ne!(empty_frame.image.as_raw(), value_frame.image.as_raw());
}

#[test]
fn vertical_bar_fills_from_the_bottom() {
    let root = temp_root("vertical-bar");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({"id":"bars","name":"Bars","enabled":true,"duration":8,"revision":1,"background":{"color":"#071019"},"layers":[{"id":"v","type":"bar","binding":"aooscope_pve_cpu_pct","x":10,"y":10,"width":20,"height":100,"z":1,"color":"#35d9ff","orientation":"vertical"}]})).unwrap();
    let state = StateDocument {
        pve: Some(json!({"cpu_pct": 50})),
        ..Default::default()
    };
    let image = compile_page(&page, &state, &media, 100, 0.0).unwrap().image;
    assert_eq!(image.get_pixel(15, 20).0, [29, 38, 50]);
    assert_eq!(image.get_pixel(15, 80).0, [53, 217, 255]);
}

#[test]
fn gauges_and_bars_use_rounded_geometry() {
    let root = temp_root("rounded-primitives");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"geometry", "name":"Geometry", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[
            {"id":"gauge","type":"gauge","binding":"aooscope_pve_cpu_pct","x":40,"y":40,"width":180,"height":180,"z":1,"color":"#35d9ff","thickness":18},
            {"id":"bar","type":"bar","binding":"aooscope_pve_cpu_pct","x":300,"y":80,"width":240,"height":24,"z":1,"color":"#35d9ff"}
        ]
    }))
    .unwrap();
    let state = StateDocument {
        pve: Some(json!({"cpu_pct": 50})),
        ..Default::default()
    };
    let image = compile_page(&page, &state, &media, 100, 0.0).unwrap().image;
    assert_eq!(image.get_pixel(40, 40).0, [7, 16, 25]);
    assert_eq!(image.get_pixel(300, 80).0, [7, 16, 25]);
    assert_eq!(image.get_pixel(130, 40).0, [53, 217, 255]);
}

#[test]
fn nested_state_bindings_and_text_units_are_rendered() {
    let root = temp_root("nested-bindings");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"nested", "name":"Nested", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[
            {"id":"disk","type":"value","binding":"aooscope_pve_smart_0_temperature_c","x":20,"y":20,"width":180,"height":32,"z":1,"color":"#ffffff","unit":" C","scale":2},
            {"id":"media","type":"text","binding":"aooscope_media_display_title_short","x":20,"y":80,"width":300,"height":32,"z":1,"color":"#ffffff","scale":2}
        ]
    }))
    .unwrap();
    let state = StateDocument {
        pve: Some(json!({"smart":[{"temperature_c": 41}]})),
        media: Some(json!({"display":{"title_short":"ORBIT"}})),
        ..Default::default()
    };
    let populated = compile_page(&page, &state, &media, 100, 0.0).unwrap();
    let empty = compile_page(&page, &StateDocument::default(), &media, 100, 0.0).unwrap();
    assert_ne!(populated.image.as_raw(), empty.image.as_raw());
    assert!(populated.warnings.is_empty());
}

#[test]
fn factory_semantics_cover_home_storage_compute_media_and_splash() {
    let root = temp_root("goldens");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    for (name, background) in [
        ("Home", "#071019"),
        ("Storage", "#071019"),
        ("Compute", "#071019"),
        ("Media", "#071019"),
        ("Splash", "#071019"),
    ] {
        let page: Page = serde_json::from_value(json!({"id":name.to_lowercase(),"name":name,"enabled":true,"duration":8,"revision":1,"background":{"color":background},"layers":[{"id":"value","type":"value","binding":"aooscope_pve_cpu_pct","x":34,"y":112,"width":212,"height":220,"z":1},{"id":"bar","type":"bar","binding":"aooscope_pve_memory_pct","x":50,"y":300,"width":160,"height":12,"z":2}]})).unwrap();
        let frame = compile_page(&page, &StateDocument::default(), &media, 100, 50.0).unwrap();
        assert_eq!(frame.image.get_pixel(0, 0).0, [7, 16, 25]);
        assert!(frame.image.pixels().any(|pixel| pixel.0 != [7, 16, 25]));
    }
}

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("aooscope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
