use aooscope_render::{MediaStore, RevisionStore};
use aooscope_types::{Page, PagesDocument};
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

fn temp_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("aooscope-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}
