use crate::ConfigError;
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ConfigError> {
    let parent = path
        .parent()
        .ok_or_else(|| ConfigError::NoParent(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
        path: parent.to_path_buf(),
        source,
    })?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("data.json");
    let temp = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|source| ConfigError::Write {
                path: temp.clone(),
                source,
            })?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|source| {
            ConfigError::Serialize {
                path: path.to_path_buf(),
                source,
            }
        })?;
        file.write_all(b"\n").map_err(|source| ConfigError::Write {
            path: temp.clone(),
            source,
        })?;
        file.sync_all().map_err(|source| ConfigError::Write {
            path: temp.clone(),
            source,
        })?;
        fs::rename(&temp, path).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })?;
        File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|source| ConfigError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
