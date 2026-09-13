use crate::CompiledDocument;
use aooscope_config::atomic_write_json;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RevisionError {
    #[error("revision not found: {0}")]
    NotFound(String),
    #[error("no previous revision")]
    NoPrevious,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config: {0}")]
    Config(#[from] aooscope_config::ConfigError),
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RevisionPointer {
    pub revision_id: String,
    pub promoted_unix: f64,
}
#[derive(Clone, Debug)]
pub struct RevisionStore {
    root: PathBuf,
}
impl RevisionStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().join("compiled");
        let _ = fs::create_dir_all(&root);
        Self { root }
    }
    fn pointer(&self, name: &str) -> Option<RevisionPointer> {
        serde_json::from_slice(&fs::read(self.root.join(name)).ok()?).ok()
    }
    pub fn current(&self) -> Option<RevisionPointer> {
        self.pointer("current.json")
    }
    pub fn previous(&self) -> Option<RevisionPointer> {
        self.pointer("previous.json")
    }
    pub fn stage(&self, doc: Value, state: Value) -> Result<String, RevisionError> {
        fs::create_dir_all(&self.root)?;
        let id = format!(
            "r{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            &Uuid::new_v4().to_string()[..8]
        );
        let dir = self.root.join(&id);
        fs::create_dir(&dir)?;
        atomic_write_json(&dir.join("source-pages.json"), &doc)?;
        atomic_write_json(&dir.join("state.json"), &state)?;
        atomic_write_json(
            &dir.join("manifest.json"),
            &serde_json::json!({"revision_id":id}),
        )?;
        Ok(id)
    }

    pub fn stage_compiled(
        &self,
        doc: Value,
        state: Value,
        compiled: &CompiledDocument,
    ) -> Result<String, RevisionError> {
        let id = self.stage(doc, state)?;
        let dir = self.root.join(&id);
        let mut warnings = Vec::new();
        let mut frames = Vec::new();
        for (index, page) in compiled.pages.iter().enumerate() {
            let name = format!("frame-{index}.png");
            page.image
                .save(dir.join(&name))
                .map_err(|e| RevisionError::Io(std::io::Error::other(e)))?;
            warnings.extend(page.warnings.clone());
            frames.push(name);
        }
        atomic_write_json(
            &dir.join("manifest.json"),
            &serde_json::json!({"revision_id":id,"warnings":warnings,"frames":frames}),
        )?;
        Ok(id)
    }
    pub fn promote(&self, id: &str) -> Result<RevisionPointer, RevisionError> {
        fs::create_dir_all(&self.root)?;
        let dir = self.root.join(id);
        if !dir.is_dir() {
            return Err(RevisionError::NotFound(id.into()));
        }
        if let Some(old) = self.current() {
            atomic_write_json(&self.root.join("previous.json"), &old)?;
        }
        let p = RevisionPointer {
            revision_id: id.into(),
            promoted_unix: unix_now(),
        };
        atomic_write_json(&self.root.join("current.json"), &p)?;
        Ok(p)
    }
    pub fn rollback(&self) -> Result<RevisionPointer, RevisionError> {
        fs::create_dir_all(&self.root)?;
        let previous = self.previous().ok_or(RevisionError::NoPrevious)?;
        let current = self.current();
        let promoted = RevisionPointer {
            revision_id: previous.revision_id,
            promoted_unix: unix_now(),
        };
        atomic_write_json(&self.root.join("current.json"), &promoted)?;
        if let Some(current) = current {
            atomic_write_json(&self.root.join("previous.json"), &current)?;
        }
        Ok(promoted)
    }
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
