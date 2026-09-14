use aooscope_render::{MediaStore, RevisionStore};
use aooscope_types::{Layer, Page, PagesDocument};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn media_replace_and_reference_protection_are_atomic() {
    let root = temp_root("media");
    let store = MediaStore::new(&root).unwrap();
    let asset = store.ingest(b"not-image", "a.bin");
    assert!(asset.is_err());
    let png = image::RgbImage::from_pixel(2, 3, image::Rgb([1, 2, 3]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(png)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let asset = store.ingest(&bytes, "a.png").unwrap();
    let page: Page = serde_json::from_value(json!({"id":"p","name":"P","duration":8,"revision":1,"layers":[{"id":"i","type":"image","asset_id":asset.id,"x":0,"y":0,"width":2,"height":3,"z":1}]})).unwrap();
    let doc = PagesDocument {
        schema_version: 1,
        revision: 1,
        carousel: vec!["p".into()],
        pages: [("p".into(), page)].into_iter().collect(),
        extra: Default::default(),
    };
    assert!(store.delete(&asset.id, &doc).is_err());
    let replaced = store.replace(&asset.id, &bytes, "b.png").unwrap();
    assert_eq!(replaced.revision, 2);
}

#[test]
fn revisions_promote_previous_and_rollback() {
    let root = temp_root("revision");
    let store = RevisionStore::new(&root);
    let one = store
        .stage(json!({"revision":1}), json!({"value":1}))
        .unwrap();
    store.promote(&one).unwrap();
    let two = store
        .stage(json!({"revision":2}), json!({"value":2}))
        .unwrap();
    store.promote(&two).unwrap();
    assert_eq!(store.current().unwrap().revision_id, two);
    assert_eq!(store.rollback().unwrap().revision_id, one);
}

#[test]
fn gif_ingest_is_supported_by_the_enabled_decoder() {
    let root = temp_root("gif");
    let store = MediaStore::new(&root).unwrap();
    let image = image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Gif,
        )
        .unwrap();
    assert_eq!(store.ingest(&bytes, "test.gif").unwrap().format, "GIF");
}

#[test]
fn orbit_preset_reuses_source_bytes_and_leaves_custom_splash_untouched() {
    let root = temp_root("orbit");
    let store = MediaStore::new(&root).unwrap();
    let image = image::RgbImage::from_pixel(2, 2, image::Rgb([9, 8, 7]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let source = store.ingest(&bytes, "logo.png").unwrap();
    let before = std::fs::read(root.join("media").join(&source.stored_name)).unwrap();
    let preset = store.ensure_orbit(&source.id, None).unwrap();
    assert_eq!(preset.id, "orbit");
    assert_eq!(preset.name, "Orbit");
    assert_eq!(preset.source_asset_id, source.id);
    assert_eq!(preset.settings["fps"], 5);
    assert_eq!(preset.settings["speed_seconds"], 4);
    assert_eq!(std::fs::read_dir(root.join("media")).unwrap().count(), 1);
    assert_eq!(
        before,
        std::fs::read(root.join("media").join(&source.stored_name)).unwrap()
    );

    let mut custom_pages = PagesDocument {
        schema_version: 1,
        revision: 1,
        carousel: vec!["splash".into()],
        pages: Default::default(),
        extra: Default::default(),
    };
    custom_pages.pages.insert(
        "splash".into(),
        serde_json::from_value(json!({
            "id":"splash", "name":"Splash", "duration":8, "revision":1,
            "layers":[{"id":"logo","type":"image","asset_id":source.id,"x":0,"y":0,"width":2,"height":2,"z":1}]
        })).unwrap(),
    );
    let existing = store.ensure_orbit(&source.id, None).unwrap();
    assert_eq!(existing, preset);
    let before = serde_json::to_vec(&custom_pages.pages["splash"]).unwrap();
    assert!(!MediaStore::migrate_splash(&mut custom_pages, &existing));
    assert_eq!(
        serde_json::to_vec(&custom_pages.pages["splash"]).unwrap(),
        before
    );

    let mut pages = PagesDocument {
        schema_version: 1,
        revision: 1,
        carousel: vec!["splash".into()],
        pages: Default::default(),
        extra: Default::default(),
    };
    pages.pages.insert(
        "splash".into(),
        Page {
            id: "splash".into(),
            name: "Splash".into(),
            enabled: true,
            duration: 8,
            template_id: None,
            revision: 1,
            background: Default::default(),
            layers: vec![Layer {
                id: "custom".into(),
                layer_type: "animation".into(),
                binding: None,
                x: 0,
                y: 0,
                width: 2,
                height: 2,
                z: 1,
                opacity: 1.0,
                clip: false,
                extra: [("asset_id".into(), json!(source.id))]
                    .into_iter()
                    .collect(),
            }],
            extra: Default::default(),
        },
    );
    assert!(!MediaStore::migrate_splash(&mut pages, &preset));
    assert_eq!(pages.pages["splash"].layers[0].layer_type, "animation");
}

#[test]
fn orbit_preset_requires_explicit_source_and_updates_stably() {
    let root = temp_root("orbit-source-matching");
    let store = MediaStore::new(&root).unwrap();
    let image = image::RgbImage::from_pixel(1, 1, image::Rgb([9, 8, 7]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    let source = store.ingest(&bytes, "source.png").unwrap();

    let preset = store
        .ensure_orbit(&source.id, Some("Custom Orbit"))
        .unwrap();

    assert_eq!(preset.source_asset_id, source.id);
    assert_eq!(preset.name, "Custom Orbit");
    assert_eq!(store.presets().unwrap().len(), 1);
    let replacement = store.ingest(&bytes, "replacement.png").unwrap();
    let updated = store.ensure_orbit(&replacement.id, None).unwrap();
    assert_eq!(updated.id, preset.id);
    assert_eq!(updated.name, "Orbit");
    assert_eq!(updated.source_asset_id, replacement.id);
}

#[test]
fn orbit_preset_rejects_unknown_source() {
    let store = MediaStore::new(temp_root("orbit-missing")).unwrap();
    assert!(store.ensure_orbit("missing", None).is_err());
}

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("aooscope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
