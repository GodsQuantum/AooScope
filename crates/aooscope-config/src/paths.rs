use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    pub root: PathBuf,
}

impl AppPaths {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    pub fn pages(&self) -> PathBuf {
        self.root.join("pages.json")
    }
    pub fn media(&self) -> PathBuf {
        self.root.join("media.json")
    }
    pub fn state(&self) -> PathBuf {
        self.root.join("state.json")
    }
    pub fn provider_secrets(&self) -> PathBuf {
        self.root.join("private/providers.json")
    }
}
