use aooscope_config::AppPaths;
use aooscope_display::SimulatedDisplayDriver;
use aooscope_render::{HEIGHT, MediaStore, WIDTH, compile_page};
use aooscope_server::{AppState, app, bootstrap, storage, templates};
use aooscope_types::{PagesDocument, StateDocument, validate_document};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{collections::BTreeSet, fs, path::PathBuf};
use tower::ServiceExt;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "aooscope-bootstrap-{name}-{}",
        uuid::Uuid::new_v4()
    ))
}

fn storage_device(index: usize) -> storage::StorageDevice {
    storage::StorageDevice {
        index,
        path: format!("/dev/disk{index}"),
        label: format!("Disk {index}"),
        kind: "disk".into(),
        total_bytes: 1_000,
        used_bytes: Some(680),
        free_bytes: Some(320),
        usage_pct: Some(68.0),
        temperature_c: Some(68.0),
        health: Some("PASSED".into()),
    }
}

#[test]
fn storage_templates_chunk_devices_with_stable_ids_and_bounded_layers() {
    for (count, expected_pages) in [(0, 0), (1, 1), (4, 1), (6, 1), (8, 2), (10, 2)] {
        let devices = (0..count).map(storage_device).collect::<Vec<_>>();
        let pages = templates::storage_pages(&devices);
        assert_eq!(pages.len(), expected_pages, "{count} devices");
        assert_eq!(
            pages
                .iter()
                .map(|page| page.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            pages.len(),
            "{count} device page ids"
        );
        assert_eq!(
            pages
                .iter()
                .map(|page| page.id.as_str())
                .collect::<Vec<_>>(),
            (0..expected_pages)
                .map(|index| if index == 0 {
                    "page-storage".into()
                } else {
                    format!("page-storage-{}", index + 1)
                })
                .collect::<Vec<String>>()
        );
        let document = PagesDocument {
            schema_version: 1,
            revision: 1,
            carousel: pages.iter().map(|page| page.id.clone()).collect(),
            pages: pages
                .iter()
                .map(|page| (page.id.clone(), page.clone()))
                .collect(),
            extra: Default::default(),
        };
        validate_document(&document).unwrap_or_else(|issues| panic!("{count}: {issues:?}"));
        for page in pages {
            assert_eq!((WIDTH as i32, HEIGHT as i32), (960, 376));
            for layer in page.layers {
                assert!(
                    layer.x >= 0 && layer.y >= 0,
                    "{}: negative bounds",
                    layer.id
                );
                assert!(
                    layer.x + layer.width as i32 <= WIDTH as i32,
                    "{}: right overflow",
                    layer.id
                );
                assert!(
                    layer.y + layer.height as i32 <= HEIGHT as i32,
                    "{}: bottom overflow",
                    layer.id
                );
                if layer.layer_type == "bar" {
                    assert_eq!(
                        layer.binding.as_deref(),
                        Some(
                            format!(
                                "aooscope_pve_disks_{}_usage_pct",
                                layer.id.strip_prefix("storage-bar-").unwrap()
                            )
                            .as_str()
                        )
                    );
                    assert!(layer.height <= 12, "{}: capacity bar is not thin", layer.id);
                    assert_eq!(layer.extra.get("format"), None, "bars do not format text");
                }
            }
        }
    }
}

#[test]
fn generic_factory_template_ids_are_available() {
    for template in [
        "factory.vertical-bars.v1",
        "factory.semi-rings.v1",
        "factory.horizontal-bars.v1",
    ] {
        let page = templates::factory_page(template, "page-template").unwrap();
        assert_eq!(page.template_id.as_deref(), Some(template));
        assert!(!page.layers.is_empty());
    }
}

#[test]
fn storage_cards_format_capacity_values_as_human_bytes() {
    let page = templates::storage_pages(&[storage_device(0)]).remove(0);
    for id in ["storage-used-0", "storage-total-0"] {
        let layer = page.layers.iter().find(|layer| layer.id == id).unwrap();
        assert_eq!(layer.extra.get("format"), Some(&json!("bytes")), "{id}");
    }
    assert!(page.layers.iter().any(|layer| {
        layer.id == "storage-separator-0" && layer.extra.get("text") == Some(&json!("/"))
    }));
    let usage = page
        .layers
        .iter()
        .find(|layer| layer.id == "storage-usage-0")
        .expect("visible usage percentage");
    assert_eq!(
        usage.binding.as_deref(),
        Some("aooscope_pve_disks_0_usage_pct")
    );
    assert_eq!(usage.extra.get("unit"), Some(&json!("%")));
}

async fn status(app: &axum::Router, path: &str) -> StatusCode {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn empty_root_bootstraps_all_simulated_api_reads() {
    let root = root("api");
    let paths = AppPaths::new(&root);
    bootstrap(&paths).unwrap();
    let router = app(AppState::new(paths).with_display_driver(SimulatedDisplayDriver::new()));

    for path in [
        "/api/health",
        "/api/status",
        "/api/pages",
        "/api/metrics",
        "/api/media",
        "/api/display/capabilities",
        "/api/settings",
    ] {
        assert_eq!(status(&router, path).await, StatusCode::OK, "{path}");
    }
    let pages: Value = serde_json::from_slice(&fs::read(root.join("pages.json")).unwrap()).unwrap();
    assert_eq!(pages["schema_version"], 1);
    assert_eq!(pages["revision"], 1);
    assert_eq!(
        pages["carousel"],
        json!([
            "page-splash",
            "page-home",
            "page-storage",
            "page-storage-m2",
            "page-compute",
            "page-media"
        ])
    );
    assert_eq!(pages["pages"]["page-splash"]["name"], "Splash");
    assert_eq!(pages["pages"]["page-home"]["name"], "Home");
    assert_eq!(pages["pages"]["page-storage"]["name"], "Storage");
    assert_eq!(pages["pages"]["page-storage-m2"]["name"], "Storage M.2");
    assert_eq!(pages["pages"]["page-compute"]["name"], "Compute");
    assert_eq!(pages["pages"]["page-media"]["name"], "Media");
    for (id, template, enabled) in [
        ("page-splash", "factory.splash.v1", true),
        ("page-home", "factory.home.v1", true),
        ("page-storage", "factory.storage.v1", false),
        ("page-storage-m2", "factory.storage-m2.v1", false),
        ("page-compute", "factory.compute.v1", true),
        ("page-media", "factory.media.v1", true),
    ] {
        assert_eq!(pages["pages"][id]["template_id"], template);
        assert_eq!(pages["pages"][id]["enabled"], enabled);
    }
    let public_templates = serde_json::to_string(&pages["pages"])
        .unwrap()
        .to_lowercase();
    for private in private_markers() {
        assert!(
            !public_templates.contains(&private),
            "private string: {private}"
        );
    }
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(root.join("media.json")).unwrap()).unwrap()["assets"],
        json!({})
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(root.join("private/providers.json")).unwrap())
            .unwrap(),
        json!({})
    );
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(root.join("private/providers.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn bootstrap_generates_storage_pages_from_available_state() {
    let root = root("adaptive-storage");
    let paths = AppPaths::new(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        paths.state(),
        serde_json::to_vec(&json!({
            "pve": {"disks": (0..8).map(|index| json!({"name": format!("Disk {index}")})).collect::<Vec<_>>()}
        }))
        .unwrap(),
    )
    .unwrap();

    bootstrap(&paths).unwrap();
    let pages: PagesDocument = serde_json::from_slice(&fs::read(paths.pages()).unwrap()).unwrap();
    assert_eq!(
        pages.carousel,
        [
            "page-splash",
            "page-home",
            "page-storage",
            "page-storage-2",
            "page-storage-m2",
            "page-compute",
            "page-media"
        ]
    );
    assert!(pages.pages["page-storage"].enabled);
    assert!(pages.pages["page-storage-2"].enabled);
    assert!(
        pages.pages["page-storage-2"]
            .layers
            .iter()
            .any(|layer| layer.binding.as_deref() == Some("aooscope_pve_disks_7_usage_pct"))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn bootstrap_is_idempotent_and_preserves_existing_documents() {
    let root = root("idempotent");
    let paths = AppPaths::new(&root);
    fs::create_dir_all(root.join("private")).unwrap();
    let existing = [
        (paths.settings(), br#"{"display":{"brightness":42},"unknown":true}"#.to_vec()),
        (paths.pages(), serde_json::to_vec(&json!({"schema_version":1,"revision":9,"carousel":["custom"],"pages":{"custom":{"id":"custom","name":"Custom","duration":8,"revision":1,"layers":[]}},"unknown":"keep"})).unwrap()),
        (paths.media(), br#"{"schema_version":1,"assets":{},"presets":{},"unknown":"keep"}"#.to_vec()),
        (paths.state(), br#"{"unknown":"keep"}"#.to_vec()),
        (paths.provider_secrets(), br#"{"custom":{"token":"keep"}}"#.to_vec()),
    ];
    for (path, bytes) in &existing {
        fs::write(path, bytes).unwrap();
    }
    #[cfg(unix)]
    let existing_mode = fs::metadata(paths.provider_secrets())
        .unwrap()
        .permissions()
        .mode();
    let before: Vec<_> = existing
        .iter()
        .map(|(path, _)| fs::read(path).unwrap())
        .collect();

    bootstrap(&paths).unwrap();
    bootstrap(&paths).unwrap();

    for ((path, _), expected) in existing.iter().zip(before) {
        assert_eq!(
            fs::read(path).unwrap(),
            expected,
            "{} changed",
            path.display()
        );
    }
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(paths.provider_secrets())
            .unwrap()
            .permissions()
            .mode(),
        existing_mode
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn factory_templates_are_deterministic_native_and_complete() {
    let root = root("factory-visuals");
    let paths = AppPaths::new(&root);
    bootstrap(&paths).unwrap();
    let pages: PagesDocument = serde_json::from_slice(&fs::read(paths.pages()).unwrap()).unwrap();
    let media = MediaStore::new(&root).unwrap();
    let state: StateDocument = serde_json::from_value(json!({
        "pve": {
            "cpu_pct": 37, "memory_pct": 62, "guests_running": 8,
            "disks": [
                {"name":"SATA-A","size_bytes":6000000000000u64,"used_bytes":4080000000000u64,"free_bytes":1920000000000u64,"usage_pct":68},
                {"name":"SATA-B","size_bytes":6000000000000u64,"used_bytes":3300000000000u64,"free_bytes":2700000000000u64,"usage_pct":55},
                {"name":"SATA-C","size_bytes":4000000000000u64,"used_bytes":2840000000000u64,"free_bytes":1160000000000u64,"usage_pct":71},
                {"name":"SATA-D","size_bytes":3000000000000u64,"used_bytes":1170000000000u64,"free_bytes":1830000000000u64,"usage_pct":39},
                {"name":"SATA-E","size_bytes":2000000000000u64,"used_bytes":1520000000000u64,"free_bytes":480000000000u64,"usage_pct":76},
                {"name":"SATA-F","size_bytes":1000000000000u64,"used_bytes":420000000000u64,"free_bytes":580000000000u64,"usage_pct":42}
            ],
            "smart": [
                {"temperature_c":31,"health":"PASSED"},{"temperature_c":34,"health":"PASSED"},
                {"temperature_c":38,"health":"PASSED"},{"temperature_c":42,"health":"PASSED"},
                {"temperature_c":45,"health":"PASSED"},{"temperature_c":49,"health":"PASSED"}
            ],
            "nvme": [
                {"name":"NVME-A","role":"POOL","temperature_c":41,"health":"HEALTHY"},
                {"name":"NVME-B","role":"CACHE","temperature_c":48,"health":"HEALTHY"},
                {"name":"NVME-C","role":"SCRATCH","temperature_c":54,"health":"HEALTHY"},
                {"name":"NVME-D","role":"SYSTEM","temperature_c":45,"health":"HEALTHY"}
            ]
        },
        "hardware": {"cpu_temp_c":57,"gpu_busy_pct":44,"gpu_gtt_pct":29,"gpu_temp_c":63},
        "media": {"display":{"mode":"playing","title":"SYNTHETIC HORIZONS","source":"MEDIA","provider_chain":["Jellyfin"],"progress_pct":68,"eta_minutes":12,"speed_bytes_s":8240000}}
    })).unwrap();
    let expected = [
        ("page-splash", "factory.splash.v1", true),
        ("page-home", "factory.home.v1", true),
        ("page-storage", "factory.storage.v1", false),
        ("page-storage-m2", "factory.storage-m2.v1", false),
        ("page-compute", "factory.compute.v1", true),
        ("page-media", "factory.media.v1", true),
    ];
    assert_eq!(pages.carousel.len(), expected.len());
    let preview_root = std::env::var_os("AOOSCOPE_WRITE_TASK2_PREVIEWS").map(|_| {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sources/Worklogs/task2-previews");
        fs::create_dir_all(&path).unwrap();
        path
    });
    let mut hashes = Vec::new();
    for ((id, template, enabled), carousel_id) in expected.into_iter().zip(&pages.carousel) {
        assert_eq!(carousel_id, id);
        let page = &pages.pages[id];
        assert_eq!(page.template_id.as_deref(), Some(template));
        assert_eq!(page.enabled, enabled, "{id}");
        assert!(!page.layers.is_empty());
        let bindings = page
            .layers
            .iter()
            .filter_map(|layer| layer.binding.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(bindings, expected_bindings(id), "{id} bindings changed");
        let frame = compile_page(page, &state, &media, 100, 25.0).unwrap();
        assert_eq!((frame.image.width(), frame.image.height()), (WIDTH, HEIGHT));
        assert!(frame.warnings.is_empty(), "{id}: {:?}", frame.warnings);
        hashes.push(fnv1a(frame.image.as_raw()));
        if let Some(preview_root) = &preview_root {
            frame
                .image
                .save(preview_root.join(format!("{}.png", id.trim_start_matches("page-"))))
                .unwrap();
        }
    }
    assert_eq!(
        hashes,
        [
            11145793668765435691,
            3933230600154408341,
            10264025068728205431,
            9390878903665389157,
            4505336165427745734,
            7364605711451741370,
        ],
        "factory visuals changed"
    );
    let public = serde_json::to_string(&pages).unwrap().to_lowercase();
    for private in private_markers() {
        assert!(!public.contains(&private), "private string: {private}");
    }
    let _ = fs::remove_dir_all(root);
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn private_markers() -> [String; 5] {
    [
        format!("cloud {}", 9),
        ["cloud", "9"].concat(),
        format!("{}.{}.", 192, 168),
        format!("{}.{}.{}.", 10, 0, 0),
        format!("{}.{}.", 172, 16),
    ]
}

fn expected_bindings(id: &str) -> BTreeSet<String> {
    let bindings: Vec<String> = match id {
        "page-splash" => vec![],
        "page-home" => [
            "aooscope_pve_cpu_pct",
            "aooscope_pve_memory_pct",
            "aooscope_hardware_cpu_temp_c",
            "aooscope_pve_guests_running",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "page-storage" => (0..6)
            .flat_map(|index| {
                [
                    format!("aooscope_pve_disks_{index}_name"),
                    format!("aooscope_pve_disks_{index}_used_bytes"),
                    format!("aooscope_pve_disks_{index}_size_bytes"),
                    format!("aooscope_pve_disks_{index}_usage_pct"),
                    format!("aooscope_pve_smart_{index}_temperature_c"),
                    format!("aooscope_pve_smart_{index}_health"),
                ]
            })
            .collect(),
        "page-storage-m2" => (0..4)
            .flat_map(|index| {
                [
                    format!("aooscope_pve_nvme_{index}_name"),
                    format!("aooscope_pve_nvme_{index}_role"),
                    format!("aooscope_pve_nvme_{index}_temperature_c"),
                    format!("aooscope_pve_nvme_{index}_health"),
                ]
            })
            .collect(),
        "page-compute" => [
            "aooscope_hardware_gpu_busy_pct",
            "aooscope_pve_cpu_pct",
            "aooscope_hardware_gpu_gtt_pct",
            "aooscope_hardware_gpu_temp_c",
            "aooscope_hardware_cpu_temp_c",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "page-media" => [
            "aooscope_media_display_poster_asset_id",
            "aooscope_media_display_headline",
            "aooscope_media_display_title_short",
            "aooscope_media_display_source",
            "aooscope_media_display_provider_chain",
            "aooscope_media_display_progress_pct",
            "aooscope_media_display_speed_bytes_s",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        _ => panic!("unknown factory page"),
    };
    bindings.into_iter().collect()
}
