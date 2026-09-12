use crate::{AppPaths, ConfigError};
use aooscope_types::{MediaDocument, PagesDocument, ProviderSecrets, Settings, StateDocument};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ConfigError> {
    let bytes = fs::read(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| ConfigError::Json {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_settings(paths: &AppPaths) -> Result<Settings, ConfigError> {
    read_json(&paths.settings())
}
pub fn load_pages(paths: &AppPaths) -> Result<PagesDocument, ConfigError> {
    read_json(&paths.pages())
}
pub fn load_media(paths: &AppPaths) -> Result<MediaDocument, ConfigError> {
    read_json(&paths.media())
}
pub fn load_state(paths: &AppPaths) -> Result<StateDocument, ConfigError> {
    read_json(&paths.state())
}
pub fn load_provider_secrets(paths: &AppPaths) -> Result<ProviderSecrets, ConfigError> {
    read_json(&paths.provider_secrets())
}
