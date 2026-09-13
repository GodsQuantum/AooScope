use aooscope_config::atomic_write_json;
use aooscope_types::{MediaAsset, MediaDocument, MediaPreset, PagesDocument};
use image::{ImageFormat, ImageReader};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("media asset not found: {0}")]
    NotFound(String),
    #[error("invalid media: {0}")]
    Invalid(String),
    #[error("asset is referenced: {0}")]
    InUse(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config: {0}")]
    Config(#[from] aooscope_config::ConfigError),
    #[error("image: {0}")]
    Image(String),
}

#[derive(Clone, Debug)]
pub struct MediaStore {
    root: PathBuf,
    media_root: PathBuf,
    max_upload: usize,
    max_pixels: u64,
}

impl MediaStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, MediaError> {
        let root = root.as_ref().to_path_buf();
        let media_root = root.join("media");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&media_root)?;
        Ok(Self {
            root,
            media_root,
            max_upload: 64 * 1024 * 1024,
            max_pixels: 32_000_000,
        })
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.root.join("media.json")
    }

    fn load(&self) -> Result<MediaDocument, MediaError> {
        let path = self.metadata_path();
        if !path.exists() {
            return Ok(MediaDocument {
                schema_version: 1,
                assets: Default::default(),
                presets: Default::default(),
                extra: Default::default(),
            });
        }
        serde_json::from_slice(&fs::read(path)?).map_err(|e| MediaError::Invalid(e.to_string()))
    }

    fn save(&self, document: &MediaDocument) -> Result<(), MediaError> {
        atomic_write_json(&self.metadata_path(), document)?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<MediaAsset>, MediaError> {
        let mut assets = self.load()?.assets.into_values().collect::<Vec<_>>();
        assets.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(assets)
    }

    pub fn get(&self, id: &str) -> Result<MediaAsset, MediaError> {
        self.load()?
            .assets
            .get(id)
            .cloned()
            .ok_or_else(|| MediaError::NotFound(id.into()))
    }

    pub fn presets(&self) -> Result<Vec<MediaPreset>, MediaError> {
        let mut presets = self.load()?.presets.into_values().collect::<Vec<_>>();
        presets.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(presets)
    }

    pub fn ensure_orbit(
        &self,
        source_asset_id: &str,
        display_name: Option<&str>,
    ) -> Result<MediaPreset, MediaError> {
        let mut document = self.load()?;
        document
            .assets
            .get(source_asset_id)
            .ok_or_else(|| MediaError::NotFound(source_asset_id.into()))?;
        let mut settings = BTreeMap::new();
        settings.insert("fps".into(), Value::from(5));
        settings.insert("speed_seconds".into(), Value::from(4));
        let preset = MediaPreset {
            id: "orbit".into(),
            name: display_name.unwrap_or("Orbit").into(),
            source_asset_id: source_asset_id.into(),
            settings,
            extra: Default::default(),
        };
        document.presets.insert(preset.id.clone(), preset.clone());
        self.save(&document)?;
        Ok(preset)
    }

    pub fn migrate_splash(pages: &mut PagesDocument, preset: &MediaPreset) -> bool {
        let Some(page) = pages.pages.values_mut().find(|page| page.name == "Splash") else {
            return false;
        };
        if page
            .layers
            .iter()
            .any(|layer| layer.layer_type == "animation")
        {
            return false;
        }
        let Some(layer) = page.layers.iter_mut().find(|layer| {
            layer.layer_type == "image"
                && layer.extra.get("asset_id").and_then(Value::as_str)
                    == Some(preset.source_asset_id.as_str())
        }) else {
            return false;
        };
        layer.layer_type = "animation".into();
        layer
            .extra
            .insert("preset_id".into(), Value::from(preset.id.as_str()));
        page.revision += 1;
        true
    }

    pub fn resolve(&self, id: &str) -> Result<PathBuf, MediaError> {
        let asset = self.get(id)?;
        let root = self.media_root.canonicalize()?;
        let candidate = root
            .join(&asset.stored_name)
            .canonicalize()
            .map_err(|_| MediaError::NotFound(id.into()))?;
        if !candidate.starts_with(&root) || !candidate.is_file() {
            return Err(MediaError::Invalid("asset path escapes media root".into()));
        }
        Ok(candidate)
    }

    pub fn ingest(&self, bytes: &[u8], filename: &str) -> Result<MediaAsset, MediaError> {
        self.write_asset(None, bytes, filename)
    }

    pub fn replace(
        &self,
        id: &str,
        bytes: &[u8],
        filename: &str,
    ) -> Result<MediaAsset, MediaError> {
        self.get(id)?;
        self.write_asset(Some(id), bytes, filename)
    }

    fn write_asset(
        &self,
        id: Option<&str>,
        bytes: &[u8],
        filename: &str,
    ) -> Result<MediaAsset, MediaError> {
        if bytes.is_empty() {
            return Err(MediaError::Invalid("empty upload".into()));
        }
        if bytes.len() > self.max_upload {
            return Err(MediaError::Invalid("upload too large".into()));
        }
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| MediaError::Invalid(e.to_string()))?;
        let image_format = reader
            .format()
            .ok_or_else(|| MediaError::Invalid("unknown raster format".into()))?;
        let (format, suffix) = format_details(image_format)?;
        let image = reader
            .decode()
            .map_err(|_| MediaError::Invalid("unsupported or corrupt media".into()))?;
        let pixels = u64::from(image.width()) * u64::from(image.height());
        if image.width() == 0 || image.height() == 0 || pixels > self.max_pixels {
            return Err(MediaError::Invalid(
                "image dimensions exceed pixel limit".into(),
            ));
        }

        let mut document = self.load()?;
        let id = id
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let revision = document
            .assets
            .get(&id)
            .map_or(1, |asset| asset.revision + 1);
        let stored_name = format!("{id}-r{revision}{suffix}");
        let target = self.media_root.join(&stored_name);
        let temporary = target.with_file_name(format!("{stored_name}.tmp"));
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &target)?;

        let mut hash = Sha256::new();
        hash.update(bytes);
        let asset = MediaAsset {
            id: id.clone(),
            name: Path::new(filename)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("asset")
                .chars()
                .take(120)
                .collect(),
            stored_name,
            kind: "image".into(),
            format: format.into(),
            width: Some(image.width()),
            height: Some(image.height()),
            size: bytes.len() as u64,
            sha256: format!("{:x}", hash.finalize()),
            revision,
            extra: Default::default(),
        };
        let old = document.assets.insert(id, asset.clone());
        if let Err(error) = self.save(&document) {
            let _ = fs::remove_file(&target);
            return Err(error);
        }
        if let Some(old) = old {
            let _ = fs::remove_file(self.media_root.join(old.stored_name));
        }
        Ok(asset)
    }

    pub fn delete(&self, id: &str, pages: &PagesDocument) -> Result<(), MediaError> {
        let mut document = self.load()?;
        let asset = document
            .assets
            .get(id)
            .cloned()
            .ok_or_else(|| MediaError::NotFound(id.into()))?;
        let pages_value = serde_json::to_value(pages)
            .map_err(|e| MediaError::Invalid(format!("cannot inspect pages: {e}")))?;
        if references_asset(&pages_value, id)
            || document
                .presets
                .values()
                .any(|preset| preset.source_asset_id == id)
        {
            return Err(MediaError::InUse(id.into()));
        }
        document.assets.remove(id);
        self.save(&document)?;
        let _ = fs::remove_file(self.media_root.join(asset.stored_name));
        Ok(())
    }
}

fn format_details(format: ImageFormat) -> Result<(&'static str, &'static str), MediaError> {
    match format {
        ImageFormat::Png => Ok(("PNG", ".png")),
        ImageFormat::Jpeg => Ok(("JPEG", ".jpg")),
        ImageFormat::WebP => Ok(("WEBP", ".webp")),
        ImageFormat::Gif => Ok(("GIF", ".gif")),
        _ => Err(MediaError::Invalid("unsupported raster format".into())),
    }
}

fn references_asset(value: &Value, asset_id: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.get("asset_id").and_then(Value::as_str) == Some(asset_id)
                || map.values().any(|value| references_asset(value, asset_id))
        }
        Value::Array(values) => values.iter().any(|value| references_asset(value, asset_id)),
        _ => false,
    }
}
