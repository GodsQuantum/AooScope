use crate::{APP_VERSION, dto::StatusDto};
use aooscope_config::{AppPaths, ConfigError, atomic_write_json, load_settings, load_state};
use aooscope_display::{
    DisplayCapabilities, DisplayDriver, DisplayError, DisplayScheduler, DisplayWorker, FrameSource,
    PromotedRevision,
};
use aooscope_render::{MediaStore, RevisionStore, compile_document};
use aooscope_types::{MediaDisplayEvent, PagesDocument};
use image::RgbImage;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppState {
    pub paths: AppPaths,
    pub device: PathBuf,
    status_tx: watch::Sender<StatusDto>,
    media_tx: watch::Sender<MediaDisplayEvent>,
    promoted_tx: watch::Sender<Option<PromotedRevision>>,
    pub display: DisplayWorker,
    pub display_capabilities: DisplayCapabilities,
    pub(crate) settings_lock: Arc<Mutex<()>>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        let device = PathBuf::from("/dev/ttyACM0");
        let initial = status_snapshot(&paths, &device);
        let (status_tx, _) = watch::channel(initial);
        let (media_tx, _) = watch::channel(MediaDisplayEvent::default());
        let promoted = RevisionStore::new(&paths.root)
            .current()
            .map(|pointer| PromotedRevision::new(pointer.revision_id));
        let (promoted_tx, _) = watch::channel(promoted);
        Self {
            paths,
            device,
            status_tx,
            media_tx,
            promoted_tx,
            display: DisplayWorker::disabled(),
            display_capabilities: DisplayCapabilities {
                power_control: false,
                ..DisplayCapabilities::default()
            },
            settings_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn with_display_driver<D: DisplayDriver + 'static>(mut self, driver: D) -> Self {
        self.display_capabilities = driver.capabilities();
        self.display = DisplayWorker::spawn(driver, 8);
        self
    }

    pub fn start_display_runtime(
        &self,
    ) -> tokio::task::JoinHandle<Result<(), aooscope_display::DisplayError>> {
        let worker = self.display.clone();
        let promoted = self.promoted_tx.subscribe();
        let source = RevisionFrameSource::new(self.paths.clone());
        tokio::spawn(DisplayScheduler::default().run(worker, source, promoted, false, 0))
    }

    pub fn promote_display(&self, revision_id: impl Into<String>) {
        self.promoted_tx
            .send_replace(Some(PromotedRevision::new(revision_id)));
    }

    #[cfg(test)]
    fn subscribe_promoted(&self) -> watch::Receiver<Option<PromotedRevision>> {
        self.promoted_tx.subscribe()
    }

    pub fn with_device(mut self, device: impl Into<PathBuf>) -> Self {
        self.device = device.into();
        let status = status_snapshot(&self.paths, &self.device);
        self.status_tx.send_replace(status);
        self
    }

    pub fn publish_status(&self, status: StatusDto) {
        self.status_tx.send_replace(status);
    }

    pub fn subscribe_status(&self) -> watch::Receiver<StatusDto> {
        self.status_tx.subscribe()
    }

    pub fn refresh_status(&self) -> StatusDto {
        let status = status_snapshot(&self.paths, &self.device);
        self.publish_status(status.clone());
        status
    }

    pub fn publish_media(&self, event: MediaDisplayEvent) {
        self.media_tx.send_replace(event);
    }

    pub fn subscribe_media(&self) -> watch::Receiver<MediaDisplayEvent> {
        self.media_tx.subscribe()
    }

    pub fn persist_media_snapshot(&self, event: &MediaDisplayEvent) -> Result<(), ConfigError> {
        self.persist_media_snapshot_with_provider_statuses(event, &Default::default())
    }

    pub fn persist_media_snapshot_with_provider_statuses(
        &self,
        event: &MediaDisplayEvent,
        statuses: &std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Result<(), ConfigError> {
        let _guard = self.settings_lock.lock().expect("settings lock poisoned");
        let path = self.paths.state();
        let mut document = load_state(&self.paths)?;
        let event_value = serde_json::to_value(event).map_err(|source| ConfigError::Serialize {
            path: path.clone(),
            source,
        })?;
        let media = document
            .media
            .get_or_insert_with(|| serde_json::Value::Object(Default::default()));
        if !media.is_object() {
            *media = serde_json::Value::Object(Default::default());
        }
        media
            .as_object_mut()
            .expect("media initialized as object")
            .insert("display".into(), event_value);
        merge_provider_statuses(&mut document, statuses);
        atomic_write_json(&path, &document)
    }

    pub fn spawn_media_polling(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let state = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let (settings, secrets) = match (
                    load_settings(&state.paths),
                    aooscope_config::load_provider_secrets(&state.paths),
                ) {
                    (Ok(settings), Ok(secrets)) => (settings, secrets),
                    _ => continue,
                };
                let (mut event, statuses) =
                    crate::providers::media::collect_media_state_with_status(&settings, &secrets)
                        .await;
                crate::providers::media::cache_live_poster(
                    &mut event,
                    &settings,
                    &secrets,
                    &state.paths.root,
                )
                .await;
                if let Err(error) =
                    state.persist_media_snapshot_with_provider_statuses(&event, &statuses)
                {
                    tracing::warn!(%error, "media state persistence failed");
                }
                state.publish_media(event);
            }
        })
    }

    pub fn spawn_telemetry_polling(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let state = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let Ok(settings) = load_settings(&state.paths) else {
                    continue;
                };
                let secrets =
                    aooscope_config::load_provider_secrets(&state.paths).unwrap_or_default();
                let telemetry = crate::providers::telemetry::collect_telemetry_state_with_paths(
                    &settings,
                    &secrets,
                    Some(&state.paths),
                )
                .await;
                if let Err(error) = state.persist_telemetry_snapshot(&telemetry) {
                    tracing::warn!(%error, "telemetry state persistence failed");
                }
            }
        })
    }

    pub fn persist_telemetry_snapshot(
        &self,
        telemetry: &aooscope_types::StateDocument,
    ) -> Result<(), ConfigError> {
        let path = self.paths.state();
        let _guard = self.settings_lock.lock().expect("settings lock poisoned");
        let mut document = load_state(&self.paths)?;
        let providers = telemetry
            .meta
            .as_ref()
            .and_then(|meta| meta.get("providers"))
            .and_then(serde_json::Value::as_object);
        if telemetry.hardware.is_some() {
            document.hardware = telemetry.hardware.clone();
        }
        if providers
            .and_then(|items| items.get("proxmox"))
            .is_some_and(|value| {
                value.get("online").and_then(serde_json::Value::as_bool) == Some(false)
                    || value.get("configured").and_then(serde_json::Value::as_bool) == Some(false)
            })
        {
            document.pve = None;
        } else if telemetry.pve.is_some() {
            document.pve = telemetry.pve.clone();
        }
        if let Some(meta) = &telemetry.meta {
            let current = document
                .meta
                .get_or_insert_with(|| serde_json::Value::Object(Default::default()));
            if !current.is_object() {
                *current = serde_json::Value::Object(Default::default());
            }
            if let (Some(target), Some(source)) = (current.as_object_mut(), meta.as_object()) {
                if let Some(source_providers) = source
                    .get("providers")
                    .and_then(serde_json::Value::as_object)
                {
                    let target_providers = target
                        .entry("providers")
                        .or_insert_with(|| serde_json::json!({}));
                    if let (Some(target_providers), Some(source_providers)) =
                        (target_providers.as_object_mut(), Some(source_providers))
                    {
                        for (name, value) in source_providers {
                            let mut value = value.clone();
                            if value.get("online").and_then(serde_json::Value::as_bool)
                                == Some(false)
                                && let Some(last_success) = target_providers
                                    .get(name)
                                    .and_then(|old| old.get("last_success"))
                            {
                                value["last_success"] = last_success.clone();
                            }
                            target_providers.insert(name.clone(), value);
                        }
                    }
                }
                for (key, value) in source {
                    if key != "providers" {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        for (key, value) in &telemetry.extra {
            document.extra.insert(key.clone(), value.clone());
        }
        if let Some(providers) = providers {
            for (name, value) in providers {
                if value.get("online").and_then(serde_json::Value::as_bool) == Some(false)
                    || value.get("configured").and_then(serde_json::Value::as_bool) == Some(false)
                {
                    document.extra.remove(name);
                }
            }
        }
        atomic_write_json(&path, &document)
    }
}

fn merge_provider_statuses(
    document: &mut aooscope_types::StateDocument,
    statuses: &std::collections::BTreeMap<String, serde_json::Value>,
) {
    if statuses.is_empty() {
        return;
    }
    let meta = document.meta.get_or_insert_with(|| serde_json::json!({}));
    if !meta.is_object() {
        *meta = serde_json::json!({});
    }
    let providers = meta
        .as_object_mut()
        .expect("meta initialized as object")
        .entry("providers")
        .or_insert_with(|| serde_json::json!({}));
    if !providers.is_object() {
        *providers = serde_json::json!({});
    }
    let providers = providers
        .as_object_mut()
        .expect("providers initialized as object");
    for (name, incoming) in statuses {
        let mut incoming = incoming.clone();
        if incoming.get("online").and_then(serde_json::Value::as_bool) != Some(true)
            && let Some(last_success) = providers
                .get(name)
                .and_then(|old| old.get("last_success"))
                .filter(|value| !value.is_null())
        {
            incoming["last_success"] = last_success.clone();
        }
        providers.insert(name.clone(), incoming);
    }
}

pub struct RevisionFrameSource {
    paths: AppPaths,
    revision_id: Option<String>,
    started: Instant,
    speed_seconds: f64,
}

impl RevisionFrameSource {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            revision_id: None,
            started: Instant::now(),
            speed_seconds: 4.0,
        }
    }

    fn animation_settings(
        &self,
        pages: &PagesDocument,
    ) -> Result<Option<(u32, f64)>, DisplayError> {
        let media = MediaStore::new(&self.paths.root)
            .map_err(|error| DisplayError::Source(format!("cannot open media store: {error}")))?;
        let presets = media
            .presets()
            .map_err(|error| DisplayError::Source(format!("cannot read media presets: {error}")))?;
        for id in &pages.carousel {
            let Some(page) = pages.pages.get(id).filter(|page| page.enabled) else {
                continue;
            };
            if let Some(layer) = page
                .layers
                .iter()
                .find(|layer| layer.layer_type == "animation")
            {
                let preset = layer
                    .extra
                    .get("preset_id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|id| presets.iter().find(|preset| preset.id == id));
                let fps = preset
                    .and_then(|preset| preset.settings.get("fps"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(u64::from(aooscope_display::DEFAULT_ANIMATION_FPS))
                    as u32;
                let speed = layer
                    .extra
                    .get("speed_seconds")
                    .and_then(serde_json::Value::as_f64)
                    .or_else(|| {
                        preset
                            .and_then(|preset| preset.settings.get("speed_seconds"))
                            .and_then(serde_json::Value::as_f64)
                    })
                    .unwrap_or(4.0)
                    .max(0.5);
                return Ok(Some((fps, speed)));
            }
        }
        Ok(None)
    }
}

impl FrameSource for RevisionFrameSource {
    fn animation_fps(&mut self, revision: &PromotedRevision) -> Result<Option<u32>, DisplayError> {
        let source_path = self
            .paths
            .root
            .join("compiled")
            .join(&revision.id)
            .join("source-pages.json");
        let pages: PagesDocument =
            serde_json::from_slice(&std::fs::read(&source_path).map_err(|error| {
                DisplayError::Source(format!("cannot read {}: {error}", source_path.display()))
            })?)
            .map_err(|error| {
                DisplayError::Source(format!("invalid {}: {error}", source_path.display()))
            })?;
        Ok(self.animation_settings(&pages)?.map(|(fps, _)| fps))
    }

    fn frame(
        &mut self,
        revision: &PromotedRevision,
        animation: bool,
    ) -> Result<RgbImage, DisplayError> {
        if self.revision_id.as_deref() != Some(&revision.id) {
            if revision.id.is_empty() || revision.id.contains(['/', '\\']) {
                return Err(DisplayError::Source("invalid revision id".into()));
            }
            self.revision_id = Some(revision.id.clone());
            self.started = Instant::now();
        }

        let source_path = self
            .paths
            .root
            .join("compiled")
            .join(&revision.id)
            .join("source-pages.json");
        let source = std::fs::read(&source_path).map_err(|error| {
            DisplayError::Source(format!("cannot read {}: {error}", source_path.display()))
        })?;
        let pages: PagesDocument = serde_json::from_slice(&source).map_err(|error| {
            DisplayError::Source(format!("invalid {}: {error}", source_path.display()))
        })?;
        if let Some((_, speed)) = self.animation_settings(&pages)? {
            self.speed_seconds = speed;
        }
        let state = load_state(&self.paths)
            .map_err(|error| DisplayError::Source(format!("cannot load state: {error}")))?;
        let settings = load_settings(&self.paths)
            .map_err(|error| DisplayError::Source(format!("cannot load settings: {error}")))?;
        let media = MediaStore::new(&self.paths.root)
            .map_err(|error| DisplayError::Source(format!("cannot open media store: {error}")))?;
        let elapsed = self.started.elapsed().as_secs_f64();
        let phase = if animation {
            (elapsed * 100.0 / self.speed_seconds) % 100.0
        } else {
            0.0
        };
        let compiled = compile_document(&pages, &state, &media, settings.display.brightness, phase)
            .map_err(|error| DisplayError::Source(format!("cannot render revision: {error}")))?;
        if compiled.order.is_empty() {
            return Err(DisplayError::Source(
                "revision has no carousel frames".into(),
            ));
        }
        let slot = (elapsed / f64::from(compiled.switch_seconds.max(1))) as usize;
        let page = compiled
            .order
            .get(slot % compiled.order.len())
            .and_then(|index| compiled.pages.get(index.saturating_sub(1)))
            .ok_or_else(|| DisplayError::Source("revision has no carousel frames".into()))?;
        Ok(page.image.clone())
    }
}

fn status_snapshot(paths: &AppPaths, device: &Path) -> StatusDto {
    let brightness = load_settings(paths)
        .map(|settings| settings.display.brightness)
        .unwrap_or(100);
    let updated_unix = load_state(paths)
        .ok()
        .and_then(|doc| doc.meta)
        .and_then(|meta| meta.get("updated_unix").and_then(|value| value.as_i64()));
    StatusDto {
        version: APP_VERSION.to_owned(),
        brightness,
        native_brightness: false,
        device_present: device.exists(),
        updated_unix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aooscope_display::FrameSource;
    use aooscope_types::StateDocument;
    use serde_json::{Value, json};
    use std::fs;

    fn fixture(name: &str) -> (AppPaths, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("aooscope-server-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("compiled/r1")).unwrap();
        fs::write(
            root.join("compiled/r1/source-pages.json"),
            serde_json::to_vec(&json!({
                "schema_version": 1, "revision": 1, "carousel": ["home"],
                "pages": {"home": {
                    "id": "home", "name": "Home", "enabled": true, "duration": 8,
                    "revision": 1, "background": {"color": "#071019"}, "layers": [{
                        "id": "value", "type": "value", "binding": "aooscope_pve_cpu_pct",
                        "x": 20, "y": 20, "width": 240, "height": 80, "z": 1,
                        "color": "#35d9ff"
                    }]
                }}
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("settings.json"),
            br#"{"display":{"brightness":100}}"#,
        )
        .unwrap();
        fs::write(root.join("state.json"), br#"{"pve":{"cpu_pct":10}}"#).unwrap();
        (AppPaths::new(&root), root)
    }

    #[test]
    fn promoted_source_renders_current_state_without_reapplying() {
        let (paths, root) = fixture("source");
        let mut source = RevisionFrameSource::new(paths);
        let revision = PromotedRevision::new("r1");
        let first = source.frame(&revision, false).unwrap();
        assert!(first.pixels().any(|pixel| pixel.0 != [7, 16, 25]));

        fs::write(root.join("state.json"), br#"{"pve":{"cpu_pct":90}}"#).unwrap();
        let second = source.frame(&revision, false).unwrap();
        assert_ne!(first.as_raw(), second.as_raw());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_telemetry_clears_only_provider_metrics_and_preserves_last_success() {
        let root = std::env::temp_dir().join(format!("aooscope-telemetry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("state.json"),
            br#"{"pve":{"cpu_pct":10},"beszel":{"systems":2},"unrelated":{"keep":true},"media":{"display":{"mode":"playing"}},"meta":{"providers":{"proxmox":{"last_success":123},"beszel":{"last_success":456}}}}"#,
        )
        .unwrap();
        let state = AppState::new(AppPaths::new(&root));
        state
            .persist_telemetry_snapshot(&StateDocument {
                meta: Some(json!({"providers": {
                    "proxmox": {"online": false, "last_success": null},
                    "beszel": {"online": false, "last_success": null}
                }})),
                ..StateDocument::default()
            })
            .unwrap();
        let saved: Value =
            serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
        assert!(saved.get("pve").is_none());
        assert!(saved.get("beszel").is_none());
        assert_eq!(saved["unrelated"]["keep"], true);
        assert_eq!(saved["media"]["display"]["mode"], "playing");
        assert_eq!(saved["meta"]["providers"]["proxmox"]["last_success"], 123);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn disabled_telemetry_providers_clear_only_their_owned_state() {
        let root =
            std::env::temp_dir().join(format!("aooscope-telemetry-all-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("state.json"), br#"{"pve":{"cpu_pct":10},"beszel":{"systems":2},"immich":{"ping":"pong"},"ollama":{"models":3},"media":{"display":{"mode":"playing"}},"custom":{"keep":true}}"#).unwrap();
        let state = AppState::new(AppPaths::new(&root));
        state
            .persist_telemetry_snapshot(&StateDocument {
                meta: Some(json!({"providers": {
                    "proxmox":{"configured":false,"enabled":false,"online":false},
                    "beszel":{"configured":false,"enabled":false,"online":false},
                    "immich":{"configured":false,"enabled":false,"online":false},
                    "ollama":{"configured":false,"enabled":false,"online":false}
                }})),
                ..StateDocument::default()
            })
            .unwrap();
        let saved: Value =
            serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
        for key in ["pve", "beszel", "immich", "ollama"] {
            assert!(saved.get(key).is_none(), "{key}");
        }
        assert_eq!(saved["media"]["display"]["mode"], "playing");
        assert_eq!(saved["custom"]["keep"], true);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn media_snapshot_persists_provider_health() {
        let root =
            std::env::temp_dir().join(format!("aooscope-media-health-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("state.json"), b"{}").unwrap();
        let state = AppState::new(AppPaths::new(&root));
        let statuses = std::collections::BTreeMap::from([(
            "jellyfin".to_owned(),
            json!({"configured": true, "enabled": true, "online": true, "last_success": 123, "error": null}),
        )]);
        state
            .persist_media_snapshot_with_provider_statuses(&MediaDisplayEvent::default(), &statuses)
            .unwrap();
        let saved: Value =
            serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
        assert_eq!(saved["meta"]["providers"]["jellyfin"]["online"], true);
        assert_eq!(saved["meta"]["providers"]["jellyfin"]["last_success"], 123);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn animated_assets_advance_by_layer_speed_while_static_pages_do_not() {
        use image::{Delay, Frame, Rgba, RgbaImage, codecs::gif::GifEncoder};

        let (paths, root) = fixture("animation");
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
        let asset = MediaStore::new(&root)
            .unwrap()
            .ingest(&bytes, "pulse.gif")
            .unwrap();
        let animation = json!({
            "schema_version": 1, "revision": 1, "carousel": ["home"],
            "pages": {"home": {
                "id": "home", "name": "Home", "enabled": true, "duration": 8,
                "revision": 1, "background": {"color": "#071019"}, "layers": [{
                    "id": "splash", "type": "animation", "asset_id": asset.id,
                    "x": 400, "y": 100, "width": 100, "height": 100, "z": 1,
                    "speed_seconds": 2
                }]
            }}
        });
        fs::write(
            root.join("compiled/r1/source-pages.json"),
            serde_json::to_vec(&animation).unwrap(),
        )
        .unwrap();
        let mut source = RevisionFrameSource::new(paths.clone());
        let revision = PromotedRevision::new("r1");
        assert_eq!(source.animation_fps(&revision).unwrap(), Some(5));
        let first = source.frame(&revision, true).unwrap();
        source.started -= Duration::from_secs(1);
        let second = source.frame(&revision, true).unwrap();
        assert_ne!(first.as_raw(), second.as_raw());

        fs::write(
            root.join("compiled/r1/source-pages.json"),
            serde_json::to_vec(&json!({
                "schema_version": 1, "revision": 1, "carousel": ["home"],
                "pages": {"home": {"id":"home","name":"Home","enabled":true,"duration":8,"revision":1,"layers":[]}}
            })).unwrap(),
        ).unwrap();
        assert_eq!(source.animation_fps(&revision).unwrap(), None);
        let static_first = source.frame(&revision, false).unwrap();
        source.started -= Duration::from_secs(1);
        assert_eq!(
            static_first.as_raw(),
            source.frame(&revision, false).unwrap().as_raw()
        );
        assert_eq!(
            DisplayScheduler::default().refresh_interval(false),
            Duration::from_secs(1)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_promoted_source_is_reported_without_panicking() {
        let (paths, root) = fixture("missing");
        let mut source = RevisionFrameSource::new(paths);
        let error = source
            .frame(&PromotedRevision::new("missing"), false)
            .unwrap_err();
        assert!(matches!(error, DisplayError::Source(_)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn app_state_restores_promoted_revision_on_restart() {
        let (paths, root) = fixture("restart");
        RevisionStore::new(&root).promote("r1").unwrap();

        let state = AppState::new(paths);

        assert_eq!(
            state
                .subscribe_promoted()
                .borrow()
                .as_ref()
                .map(|revision| revision.id.as_str()),
            Some("r1")
        );
        let _ = fs::remove_dir_all(root);
    }
}
