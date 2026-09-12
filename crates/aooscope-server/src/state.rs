use crate::{APP_VERSION, dto::StatusDto};
use aooscope_config::{AppPaths, load_settings, load_state};
use std::path::{Path, PathBuf};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub device: PathBuf,
    status_tx: watch::Sender<StatusDto>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        let device = PathBuf::from("/dev/ttyACM0");
        let initial = status_snapshot(&paths, &device);
        let (status_tx, _) = watch::channel(initial);
        Self {
            paths,
            device,
            status_tx,
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
