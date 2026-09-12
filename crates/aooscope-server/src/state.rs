use aooscope_config::AppPaths;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub device: PathBuf,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            device: PathBuf::from("/dev/ttyACM0"),
        }
    }

    pub fn with_device(mut self, device: impl Into<PathBuf>) -> Self {
        self.device = device.into();
        self
    }
}
