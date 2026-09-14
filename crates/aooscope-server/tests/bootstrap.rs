use aooscope_config::AppPaths;
use aooscope_display::SimulatedDisplayDriver;
use aooscope_render::{HEIGHT, MediaStore, WIDTH, compile_page};
use aooscope_server::{AppState, app, bootstrap};
use aooscope_types::{PagesDocument, StateDocument};
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
        ("page-storage", "factory.storage.v1", true),
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
                {"name":"SATA-A"},{"name":"SATA-B"},{"name":"SATA-C"},
                {"name":"SATA-D"},{"name":"SATA-E"},{"name":"SATA-F"}
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
        ("page-storage", "factory.storage.v1", true),
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
        assert_eq!(page.enabled, enabled);
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
            5412065298413620470,
            16648500987771008866,
            3299186535017804767,
            17943238609262544446,
            10646309534052772671,
            10206984733331649195,
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
            "aooscope_media_display_eta_minutes",
            "aooscope_media_display_speed_bytes_s",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        _ => panic!("unknown factory page"),
    };
    bindings.into_iter().collect()
}
