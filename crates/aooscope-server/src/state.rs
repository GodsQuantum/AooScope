use crate::{APP_VERSION, dto::StatusDto};
use aooscope_config::{AppPaths, ConfigError, atomic_write_json, load_settings, load_state};
use aooscope_display::{
    DisplayDriver, DisplayError, DisplayScheduler, DisplayWorker, FrameSource, PromotedRevision,
};
use aooscope_render::{MediaStore, RevisionStore, compile_document};
use aooscope_types::{MediaDisplayEvent, PagesDocument};
use image::RgbImage;
use std::path::{Path, PathBuf};
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
        }
    }

    pub fn with_display_driver<D: DisplayDriver + 'static>(mut self, driver: D) -> Self {
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
                let event = crate::providers::media::collect_media_state(&settings, &secrets).await;
                if let Err(error) = state.persist_media_snapshot(&event) {
                    tracing::warn!(%error, "media state persistence failed");
                }
                state.publish_media(event);
            }
        })
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
    use serde_json::json;
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
    fn orbit_frames_advance_by_layer_speed_while_static_frames_do_not() {
        let (paths, root) = fixture("animation");
        let animation = json!({
            "schema_version": 1, "revision": 1, "carousel": ["home"],
            "pages": {"home": {
                "id": "home", "name": "Home", "enabled": true, "duration": 8,
                "revision": 1, "background": {"color": "#071019"}, "layers": [{
                    "id": "orbit", "type": "animation", "x": 400, "y": 100,
                    "width": 100, "height": 100, "z": 1, "speed_seconds": 2
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
