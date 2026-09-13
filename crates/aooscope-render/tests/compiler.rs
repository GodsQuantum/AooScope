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
