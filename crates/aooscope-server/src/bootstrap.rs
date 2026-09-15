use crate::{storage::inventory, templates};
use aooscope_config::{AppPaths, ConfigError, atomic_write_json, atomic_write_private_json};
use aooscope_types::{MediaDocument, PagesDocument, ProviderSecrets, Settings, StateDocument};
use std::{collections::BTreeMap, fs};

pub fn bootstrap(paths: &AppPaths) -> Result<(), ConfigError> {
    fs::create_dir_all(paths.root.join("media")).map_err(|source| ConfigError::Write {
        path: paths.root.join("media"),
        source,
    })?;
    fs::create_dir_all(paths.root.join("private")).map_err(|source| ConfigError::Write {
        path: paths.root.join("private"),
        source,
    })?;

    let devices = if !paths.pages().exists() && paths.state().exists() {
        inventory(&aooscope_config::load_state(paths)?)
    } else {
        Vec::new()
    };
    let mut pages = BTreeMap::new();
    for (id, template) in [
        ("page-splash", "factory.splash.v1"),
        ("page-home", "factory.home.v1"),
        ("page-storage-m2", "factory.storage-m2.v1"),
        ("page-compute", "factory.compute.v1"),
        ("page-media", "factory.media.v1"),
    ] {
        pages.insert(
            id.to_owned(),
            templates::factory_page(template, id).expect("known factory page"),
        );
    }
    let mut storage_pages = templates::storage_pages(&devices);
    if storage_pages.is_empty() {
        let mut page = templates::factory_page("factory.storage.v1", "page-storage")
            .expect("known factory page");
        page.enabled = false;
        storage_pages.push(page);
    }
    let storage_ids = storage_pages
        .iter()
        .map(|page| page.id.clone())
        .collect::<Vec<_>>();
    pages.extend(
        storage_pages
            .into_iter()
            .map(|page| (page.id.clone(), page)),
    );
    let mut carousel = vec!["page-splash".into(), "page-home".into()];
    carousel.extend(storage_ids);
    carousel.extend(
        ["page-storage-m2", "page-compute", "page-media"]
            .into_iter()
            .map(str::to_owned),
    );
    let pages = PagesDocument {
        schema_version: 1,
        revision: 1,
        carousel,
        pages,
        extra: BTreeMap::new(),
    };
    let settings = Settings {
        display: Default::default(),
        providers: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let media = MediaDocument {
        schema_version: 1,
        assets: BTreeMap::new(),
        presets: BTreeMap::new(),
        extra: BTreeMap::new(),
    };
    let state = StateDocument::default();
    let secrets = ProviderSecrets::new();

    for (path, value) in [
        (paths.settings(), serde_json::to_value(settings).unwrap()),
        (paths.pages(), serde_json::to_value(pages).unwrap()),
        (paths.media(), serde_json::to_value(media).unwrap()),
        (paths.state(), serde_json::to_value(state).unwrap()),
        (
            paths.provider_secrets(),
            serde_json::to_value(secrets).unwrap(),
        ),
    ] {
        if !path.exists() {
            if path == paths.provider_secrets() {
                atomic_write_private_json(&path, &value)?;
            } else {
                atomic_write_json(&path, &value)?;
            }
        }
    }
    Ok(())
}
