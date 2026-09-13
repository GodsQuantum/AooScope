use crate::{APP_VERSION, dto::StatusDto};
use aooscope_config::{AppPaths, ConfigError, atomic_write_json, load_settings, load_state};
use aooscope_types::MediaDisplayEvent;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub device: PathBuf,
    status_tx: watch::Sender<StatusDto>,
    media_tx: watch::Sender<MediaDisplayEvent>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        let device = PathBuf::from("/dev/ttyACM0");
        let initial = status_snapshot(&paths, &device);
        let (status_tx, _) = watch::channel(initial);
        let (media_tx, _) = watch::channel(MediaDisplayEvent::default());
        Self {
            paths,
            device,
            status_tx,
            media_tx,
        }
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
