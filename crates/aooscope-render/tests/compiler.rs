use aooscope_render::typography::{
    HorizontalAlign, TextStyle, VerticalAlign, draw_text, measure_text,
};
use aooscope_render::{HEIGHT, WIDTH, compile_document, compile_page};
use aooscope_types::{Page, PagesDocument, StateDocument};
use image::{Rgb, RgbImage};
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
fn typography_renders_unicode_with_alignment_ellipsis_and_antialiasing() {
    let background = Rgb([7, 16, 25]);
    let mut image = RgbImage::from_pixel(WIDTH, HEIGHT, background);
    draw_text(
        &mut image,
        (20, 20, 300, 56),
        "Température 42 °C",
        TextStyle {
            pixel_size: 24.0,
            color: Rgb([244, 251, 255]),
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Center,
            max_lines: Some(1),
            ellipsis: true,
        },
    );
    let measured = measure_text(
        "Température 42 °C",
        TextStyle {
            pixel_size: 24.0,
            ..TextStyle::default()
        },
    );
    assert!(measured.width > 0.0 && measured.height > 0.0);
    draw_text(
        &mut image,
        (400, 20, 300, 56),
        "right aligned",
        TextStyle {
            horizontal_align: HorizontalAlign::Right,
            ..TextStyle::default()
        },
    );
    let mut ellipsis = RgbImage::from_pixel(WIDTH, HEIGHT, background);
    let mut clipped = RgbImage::from_pixel(WIDTH, HEIGHT, background);
    let narrow_bounds = (20, 100, 140, 40);
    let narrow_style = TextStyle {
        pixel_size: 24.0,
        color: Rgb([244, 251, 255]),
        max_lines: Some(1),
        ellipsis: true,
        ..TextStyle::default()
    };
    draw_text(
        &mut ellipsis,
        narrow_bounds,
        "Température 42 °C",
        narrow_style,
    );
    draw_text(
        &mut clipped,
        narrow_bounds,
        "Température 42 °C",
        TextStyle {
            ellipsis: false,
            ..narrow_style
        },
    );
    assert_ne!(ellipsis.as_raw(), clipped.as_raw());
    assert_eq!((image.width(), image.height()), (WIDTH, HEIGHT));
    assert!(image.pixels().any(|pixel| {
        pixel != &background && pixel.0.iter().any(|channel| *channel > 7 && *channel < 244)
    }));
}

#[test]
fn compiler_defaults_to_bounded_ellipsis_and_respects_override() {
    let root = temp_root("text-default-ellipsis");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let long = "A very long media title that cannot fit inside this narrow LCD text box";
    let mut bounded: Page = serde_json::from_value(json!({
        "id":"bounded","name":"Bounded","enabled":true,"duration":8,"revision":1,
        "background":{"color":"#071019"},
        "layers":[{
            "id":"title","type":"text","x":20,"y":20,"width":120,"height":28,"z":1,
            "text":long,"color":"#f4fbff","size":24
        }]
    }))
    .unwrap();
    let default_frame = compile_page(&bounded, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    bounded.layers[0]
        .extra
        .insert("ellipsis".into(), json!(false));
    let clipped_frame = compile_page(&bounded, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    assert_ne!(default_frame.as_raw(), clipped_frame.as_raw());

    bounded.layers[0]
        .extra
        .insert("ellipsis".into(), json!(true));
    bounded.layers[0].extra.insert("max_lines".into(), json!(1));
    let explicit_frame = compile_page(&bounded, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    assert_eq!(default_frame.as_raw(), explicit_frame.as_raw());
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
fn horizontal_storage_bar_uses_usage_not_temperature() {
    let root = temp_root("horizontal-storage-bar");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"storage", "name":"Storage", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[
            {"id":"bar","type":"bar","binding":"aooscope_pve_disks_0_usage_pct","x":10,"y":10,"width":200,"height":8,"z":1,"color":"#35d9ff","track_color":"#1a2a39","radius":4},
            {"id":"temperature","type":"value","binding":"aooscope_pve_smart_0_temperature_c","x":10,"y":30,"width":100,"height":20,"z":1}
        ]
    })).unwrap();
    let state = StateDocument {
        pve: Some(json!({
            "disks":[{"usage_pct":68.0}],
            "smart":[{"temperature_c":20.0}]
        })),
        ..Default::default()
    };
    let image = compile_page(&page, &state, &media, 100, 0.0).unwrap().image;
    let fill = (0..200)
        .filter(|offset| image.get_pixel(10 + offset, 13).0 == [53, 217, 255])
        .count();
    assert!((130..=140).contains(&fill), "usage fill length: {fill}");
    assert_eq!(image.get_pixel(10 + 150, 13).0, [26, 42, 57]);
}

#[test]
fn storage_byte_bindings_resolve_canonical_normalized_fields() {
    let root = temp_root("storage-byte-bindings");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"storage-bytes", "name":"Storage bytes", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[
            {"id":"used","type":"value","binding":"aooscope_pve_disks_0_used_bytes","x":20,"y":20,"width":160,"height":30,"z":1,"color":"#ffffff","format":"bytes","font_size":22},
            {"id":"total","type":"value","binding":"aooscope_pve_disks_0_size_bytes","x":200,"y":20,"width":160,"height":30,"z":1,"color":"#ffffff","format":"bytes","font_size":22}
        ]
    })).unwrap();
    let state = StateDocument {
        pve: Some(
            json!({"disks":[{"used_bytes":4_080_000_000_000u64,"size_bytes":6_000_000_000_000u64}]}),
        ),
        ..Default::default()
    };
    let populated = compile_page(&page, &state, &media, 100, 0.0).unwrap().image;
    let empty = compile_page(&page, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    assert_ne!(populated.as_raw(), empty.as_raw());
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
fn media_headline_renders_human_eta_and_playback_remaining_time() {
    let root = temp_root("media-headline");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let dynamic: Page = serde_json::from_value(json!({
        "id":"media", "name":"Media", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[{"id":"headline","type":"text","binding":"aooscope_media_display_headline","x":20,"y":20,"width":500,"height":64,"z":1,"color":"#62e3a3","font_size":42}]
    })).unwrap();
    let static_page = |text: &str| -> Page {
        serde_json::from_value(json!({
            "id":"expected", "name":"Expected", "enabled":true, "duration":8, "revision":1,
            "background":{"color":"#071019"},
            "layers":[{"id":"headline","type":"text","text":text,"x":20,"y":20,"width":500,"height":64,"z":1,"color":"#62e3a3","font_size":42}]
        })).unwrap()
    };
    for (display, expected) in [
        (json!({"mode":"incoming","eta_minutes":7}), "READY IN 7 MIN"),
        (
            json!({"mode":"playing","remaining_minutes":18}),
            "18 MIN LEFT",
        ),
    ] {
        let state = StateDocument {
            media: Some(json!({"display":display})),
            ..Default::default()
        };
        let rendered = compile_page(&dynamic, &state, &media, 100, 0.0)
            .unwrap()
            .image;
        let expected = compile_page(&static_page(expected), &state, &media, 100, 0.0)
            .unwrap()
            .image;
        assert_eq!(rendered.as_raw(), expected.as_raw());
    }
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

#[test]
fn full_ring_endpoint_is_filled_without_track_seam() {
    let root = temp_root("full-ring");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"ring100", "name":"Ring 100", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[{"id":"ring","type":"ring","x":40,"y":40,"width":180,"height":180,"z":1,"color":"#35d9ff","track_color":"#1d2632","thickness":18,"value":100}]
    })).unwrap();
    let image = compile_page(&page, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    assert_eq!(image.get_pixel(130, 49).0, [53, 217, 255]);
}

#[test]
fn animation_layers_render_asset_frames_without_synthetic_orbit_marker() {
    use image::{Delay, Frame, Rgba, RgbaImage, codecs::gif::GifEncoder};
    let root = temp_root("animated-asset");
    let media = aooscope_render::MediaStore::new(&root).unwrap();
    let mut bytes = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut bytes);
        encoder
            .encode_frames([
                Frame::from_parts(
                    RgbaImage::from_pixel(2, 2, Rgba([220, 20, 20, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(100, 1),
                ),
                Frame::from_parts(
                    RgbaImage::from_pixel(2, 2, Rgba([20, 220, 20, 255])),
                    0,
                    0,
                    Delay::from_numer_denom_ms(100, 1),
                ),
            ])
            .unwrap();
    }
    let asset = media.ingest(&bytes, "pulse.gif").unwrap();
    let page: Page = serde_json::from_value(json!({
        "id":"animation", "name":"Animation", "enabled":true, "duration":8, "revision":1,
        "background":{"color":"#071019"},
        "layers":[{"id":"anim","type":"animation","asset_id":asset.id,"x":100,"y":80,"width":120,"height":120,"z":1}]
    })).unwrap();
    let first = compile_page(&page, &StateDocument::default(), &media, 100, 0.0)
        .unwrap()
        .image;
    let second = compile_page(&page, &StateDocument::default(), &media, 100, 60.0)
        .unwrap()
        .image;
    assert_ne!(first.as_raw(), second.as_raw());
    assert!(!first.pixels().any(|pixel| pixel.0 == [53, 217, 255]));
    assert!(!second.pixels().any(|pixel| pixel.0 == [53, 217, 255]));
}

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("aooscope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
