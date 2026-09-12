mod atomic;
mod load;
mod paths;

use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub use atomic::atomic_write_json;
pub use load::{load_media, load_pages, load_provider_secrets, load_settings, load_state};
pub use paths::AppPaths;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot serialize {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("path has no parent: {0}")]
    NoParent(PathBuf),
}
