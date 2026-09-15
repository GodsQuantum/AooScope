use aooscope_server::providers::media::{
    cache_live_poster_bytes, fuse_media_states, merge_incoming, normalize_jelly_latest,
    normalize_jelly_sessions, normalize_qbit_torrents, normalize_radarr_queue,
    normalize_silo_sessions, normalize_sonarr_queue, select_display_event,
};
use aooscope_types::{MediaDisplayEvent, MediaMode};
use serde_json::json;

#[test]
fn jellyfin_playing_and_latest_are_normalized() {
    let playing = normalize_jelly_sessions(
        &json!([{
            "UserName":"Demo User","Client":"Jellyfin Media Player",
            "NowPlayingItem":{"Id":"m1","Name":"Dune: Part Two","RunTimeTicks":1000000000,"ProductionYear":2024},
            "PlayState":{"PositionTicks":730000000,"PlayMethod":"DirectPlay","IsPaused":false}
        }]),
        "jellyfin",
        Some("http://media.test:8096"),
    );
    assert_eq!(playing.mode, MediaMode::Playing);
    assert_eq!(playing.title.as_deref(), Some("Dune: Part Two"));
    assert_eq!(playing.progress_pct, Some(73.0));
    assert_eq!(playing.remaining_minutes, Some(1));
    assert_eq!(playing.play_method.as_deref(), Some("directplay"));
    assert!(
        playing
            .poster_url
            .as_deref()
            .unwrap()
            .contains("/Items/m1/Images/Primary")
    );

    let latest = normalize_jelly_latest(
        &json!([{"Id":"m2","Name":"Arrival","Type":"Movie","ProductionYear":2016}]),
        Some("http://media.test:8096"),
    );
    assert_eq!(latest.mode, MediaMode::Landed);
    assert_eq!(latest.title.as_deref(), Some("Arrival"));
}

#[test]
fn silo_native_session_is_normalized() {
    let event = normalize_silo_sessions(
        &json!([{
            "media_title":"Blade Runner 2049","file_duration":600.0,"position_seconds":150.0,
            "poster_url":"/poster/1","effective_play_method":"direct","client_label":"Living Room",
            "source_video_resolution":"4K","source_video_codec":"hevc","source_audio_codec":"eac3"
        }]),
        Some("http://silo.test:8091"),
    );
    assert_eq!(event.mode, MediaMode::Playing);
    assert_eq!(event.progress_pct, Some(25.0));
    assert_eq!(event.remaining_minutes, Some(8));
    assert_eq!(event.quality.as_deref(), Some("4K"));
    assert_eq!(event.provider_chain, vec!["Silo"]);
}

#[test]
fn old_media_json_remains_compatible_with_optional_enrichment() {
    let event: MediaDisplayEvent = serde_json::from_value(json!({
        "mode":"playing", "title":"Legacy", "progress_pct":50
    }))
    .unwrap();
    assert_eq!(event.remaining_minutes, None);
    assert_eq!(event.poster_asset_id, None);
    let serialized = serde_json::to_value(event).unwrap();
    assert!(serialized.get("remaining_minutes").is_none());
    assert!(serialized.get("poster_asset_id").is_none());
}

#[test]
fn arr_and_qbit_incoming_are_normalized_and_merged() {
    let radarr = normalize_radarr_queue(&json!({"records":[{
        "movie":{"title":"Dune: Part Two","images":[{"coverType":"poster","remoteUrl":"https://img.test/dune.jpg"}]},
        "size":1000.0,"sizeleft":390.0,"timeleft":"00:08:01","downloadId":"ABC","status":"downloading"
    }]}));
    assert_eq!(radarr.mode, MediaMode::Incoming);
    assert_eq!(radarr.progress_pct, Some(61.0));
    assert_eq!(radarr.eta_minutes, Some(9));
    let sonarr = normalize_sonarr_queue(&json!({"records":[{
        "series":{"title":"Severance","images":[{"coverType":"poster","remoteUrl":"https://img.test/s.jpg"}]},
        "episode":{"title":"Cold Harbor","seasonNumber":2,"episodeNumber":10},
        "size":2000.0,"sizeleft":1000.0,"timeleft":"00:05:00","downloadId":"XYZ"
    }]}));
    assert_eq!(
        sonarr.title.as_deref(),
        Some("Severance · S02E10 · Cold Harbor")
    );
    assert_eq!(sonarr.progress_pct, Some(50.0));

    let qbit = normalize_qbit_torrents(&json!([{
        "name":"Dune.Part.Two","progress":0.64,"dlspeed":44000000,"eta":480,"state":"downloading","hash":"abc"
    }]));
    let merged = merge_incoming(&radarr, &qbit);
    assert_eq!(merged.progress_pct, Some(64.0));
    assert_eq!(merged.speed_bytes_s, Some(44_000_000));
    assert_eq!(merged.eta_minutes, Some(8));
    assert_eq!(merged.provider_chain, vec!["Radarr", "qBittorrent"]);
}

#[test]
fn provider_pipeline_fuses_sonarr_with_matching_qbit_eta() {
    let sonarr = normalize_sonarr_queue(&json!({"records":[{
        "series":{"title":"Severance"}, "episode":{"title":"Cold Harbor"},
        "size":2000.0, "sizeleft":1000.0, "timeleft":"00:12:00", "downloadId":"XYZ"
    }]}));
    let qbit = normalize_qbit_torrents(&json!([{
        "name":"Severance.S02E10", "progress":0.75, "dlspeed":2400000,
        "eta":420, "state":"downloading", "hash":"xyz"
    }]));
    let merged = fuse_media_states(vec![sonarr, qbit]);
    assert_eq!(merged.provider_chain, vec!["Sonarr", "qBittorrent"]);
    assert_eq!(merged.progress_pct, Some(75.0));
    assert_eq!(merged.speed_bytes_s, Some(2_400_000));
    assert_eq!(merged.eta_minutes, Some(7));
}

#[test]
fn live_poster_cache_is_bounded_deterministic_and_private() {
    use aooscope_render::MediaStore;
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 3, image::Rgb([1, 2, 3])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let root = std::env::temp_dir().join(format!(
        "aooscope-live-poster-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let secret = "do-not-persist-this-key";

    let mut event = normalize_radarr_queue(&json!({"records":[{
        "movie":{"title":"Arrival","images":[{"coverType":"poster","remoteUrl":format!("https://media.test/poster?api_key={secret}")}]}
    }]}));
    cache_live_poster_bytes(&mut event, &root, &png);
    let first = event.poster_asset_id.clone().unwrap();
    assert_eq!(
        event.poster_url.as_deref(),
        Some("/api/media/live-poster/file")
    );
    event.poster_url = Some(format!("https://media.test/poster?api_key={secret}"));
    cache_live_poster_bytes(&mut event, &root, &png);
    assert_eq!(event.poster_asset_id.as_deref(), Some(first.as_str()));
    let assets = MediaStore::new(&root).unwrap().list().unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, "live-poster");
    assert_eq!(assets[0].revision, 2);
    assert!(!serde_json::to_string(&event).unwrap().contains(secret));
    assert!(
        !std::fs::read_to_string(root.join("media.json"))
            .unwrap()
            .contains(secret)
    );

    event.poster_url = Some(format!("https://media.test/oversized?api_key={secret}"));
    cache_live_poster_bytes(&mut event, &root, &vec![0_u8; 4 * 1024 * 1024 + 1]);
    assert_eq!(event.poster_asset_id, None);
    assert_eq!(event.poster_url, None);
    assert_eq!(MediaStore::new(&root).unwrap().list().unwrap().len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn display_priority_is_playing_then_incoming_then_landed() {
    let playing = normalize_jelly_sessions(
        &json!([{"NowPlayingItem":{"Name":"Playing","RunTimeTicks":100},"PlayState":{"PositionTicks":50}}]),
        "jellyfin",
        None,
    );
    let incoming =
        normalize_qbit_torrents(&json!([{"name":"Incoming","progress":0.5,"state":"downloading"}]));
    let landed = normalize_jelly_latest(&json!([{"Name":"Landed"}]), None);
    assert_eq!(
        select_display_event(&[landed.clone(), incoming.clone(), playing.clone()]).mode,
        MediaMode::Playing
    );
    assert_eq!(
        select_display_event(&[landed, incoming]).mode,
        MediaMode::Incoming
    );
}

#[test]
fn persisting_media_snapshot_preserves_existing_state_fields() {
    use aooscope_config::AppPaths;
    use aooscope_server::AppState;
    use aooscope_types::{MediaDisplayEvent, MediaMode};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "aooscope-media-state-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("state.json"),
        r#"{"hardware":{"keep":1},"media":{"existing":{"keep":1}},"meta":{"updated_unix":7},"custom":{"keep":true}}"#,
    )
    .unwrap();

    let state = AppState::new(AppPaths::new(&root));
    let event = MediaDisplayEvent {
        mode: MediaMode::Playing,
        source: Some("jellyfin".into()),
        title: Some("Demo Movie".into()),
        ..MediaDisplayEvent::default()
    };
    state.persist_media_snapshot(&event).unwrap();

    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("state.json")).unwrap()).unwrap();
    assert_eq!(saved["hardware"]["keep"], 1);
    assert_eq!(saved["custom"]["keep"], true);
    assert_eq!(saved["media"]["existing"]["keep"], 1);
    assert_eq!(saved["media"]["display"]["title"], "Demo Movie");
    let _ = fs::remove_dir_all(root);
}
