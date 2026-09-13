use aooscope_config::{
    AppPaths, atomic_write_json, atomic_write_private_json, load_media, load_pages,
    load_provider_secrets, load_settings, load_state,
};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/appdata-v1")
}

fn hash_file(path: &Path) -> u64 {
    let bytes = fs::read(path).unwrap();
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn fixture_files(root: &Path) -> Vec<PathBuf> {
    [
        "settings.json",
        "pages.json",
        "media.json",
        "state.json",
        "private/providers.json",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .collect()
}

#[test]
fn current_appdata_loads_without_rewriting_files() {
    let root = fixture_root();
    let before: Vec<_> = fixture_files(&root)
        .into_iter()
        .map(|path| (path.clone(), hash_file(&path)))
        .collect();
    let paths = AppPaths::new(&root);

    let settings = load_settings(&paths).unwrap();
    let pages = load_pages(&paths).unwrap();
    let media = load_media(&paths).unwrap();
    let state = load_state(&paths).unwrap();
    let secrets = load_provider_secrets(&paths).unwrap();

    assert_eq!(settings.display.brightness, 100);
    assert_eq!(pages.revision, 3);
    assert_eq!(media.schema_version, 1);
    assert!(state.meta.is_some());
    assert!(secrets.contains_key("proxmox"));
    assert!(secrets.contains_key("jellyfin"));
    for (path, expected) in before {
        assert_eq!(
            hash_file(&path),
            expected,
            "loader rewrote {}",
            path.display()
        );
    }
}

#[test]
fn unknown_fields_survive_typed_round_trip() {
    let paths = AppPaths::new(fixture_root());
    let settings = load_settings(&paths).unwrap();
    let encoded = serde_json::to_value(settings).unwrap();
    assert_eq!(encoded["display"]["timezone"], "UTC");
    let pages = serde_json::to_value(load_pages(&paths).unwrap()).unwrap();
    assert_eq!(pages["pages"]["page-home"]["layers"][0]["fallback"], "--");
}

#[test]
fn atomic_write_replaces_json_without_temp_residue() {
    let root = std::env::temp_dir().join(format!("aooscope-config-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let target = root.join("settings.json");
    let paths = AppPaths::new(fixture_root());
    let mut settings = load_settings(&paths).unwrap();
    settings.display.brightness = 73;

    atomic_write_json(&target, &settings).unwrap();
    let written: serde_json::Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(written["display"]["brightness"], 73);
    let residue: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(residue.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn private_atomic_rewrite_stays_mode_0600() {
    let root = std::env::temp_dir().join(format!("aooscope-private-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let target = root.join("providers.json");

    atomic_write_private_json(&target, &serde_json::json!({"token": "secret"})).unwrap();
    atomic_write_private_json(&target, &serde_json::json!({"token": "changed"})).unwrap();

    assert_eq!(
        fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::remove_dir_all(root).unwrap();
}
