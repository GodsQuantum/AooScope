use super::http::{HttpClient, HttpError, bounded_body, cookie_from_response};
use aooscope_types::{
    MediaDisplayEvent, MediaMode, ProviderSecrets, ProviderSettings, Settings, provider_name,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

const LIVE_POSTER_ASSET_ID: &str = "live-poster";
const MAX_POSTER_BYTES: usize = 4 * 1024 * 1024;

const ACTIVE_QBIT_STATES: &[&str] = &[
    "downloading",
    "forcedDL",
    "stalledDL",
    "metaDL",
    "checkingDL",
    "allocating",
];

fn event(mode: MediaMode, source: &str) -> MediaDisplayEvent {
    MediaDisplayEvent {
        mode,
        source: Some(source.to_owned()),
        provider_chain: vec![provider_name(source)],
        ..MediaDisplayEvent::default()
    }
}

fn absolute_url(base_url: Option<&str>, value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_owned());
    }
    base_url.map(|base| {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            value.trim_start_matches('/')
        )
    })
}
fn item_poster(base_url: Option<&str>, id: Option<&str>) -> Option<String> {
    let id = id?;
    let base = base_url?;
    Some(format!(
        "{}/Items/{id}/Images/Primary?maxWidth=360&quality=90",
        base.trim_end_matches('/')
    ))
}

fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn rounded_pct(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn remaining_minutes(duration: f64, position: f64, units_per_second: f64) -> Option<u64> {
    (duration > 0.0 && units_per_second > 0.0)
        .then(|| (((duration - position).max(0.0) / units_per_second) / 60.0).ceil() as u64)
}

pub fn normalize_jelly_sessions(
    sessions: &Value,
    source: &str,
    base_url: Option<&str>,
) -> MediaDisplayEvent {
    let Some(session) = sessions.as_array().and_then(|items| {
        items.iter().find(|item| {
            item.get("NowPlayingItem")
                .is_some_and(|value| !value.is_null())
        })
    }) else {
        return MediaDisplayEvent::idle(source);
    };
    let item = session.get("NowPlayingItem").unwrap_or(&Value::Null);
    let play = session.get("PlayState").unwrap_or(&Value::Null);
    let runtime = number(item.get("RunTimeTicks")).unwrap_or(0.0);
    let position = number(play.get("PositionTicks")).unwrap_or(0.0);
    let progress_pct = (runtime > 0.0).then(|| rounded_pct(position * 100.0 / runtime));
    let mut play_method = text(play.get("PlayMethod")).map(str::to_lowercase);
    if session
        .get("TranscodingInfo")
        .is_some_and(|value| !value.is_null())
    {
        play_method = Some("transcode".into());
    } else if play_method.is_none() {
        play_method = Some("direct".into());
    }
    let item_id = text(item.get("Id"));
    let title = text(item.get("Name")).or_else(|| text(item.get("SeriesName")));
    MediaDisplayEvent {
        title: Some(title.unwrap_or("Playing").to_owned()),
        poster_url: item_poster(base_url, item_id),
        progress_pct,
        remaining_minutes: remaining_minutes(runtime, position, 10_000_000.0),
        paused: Some(
            play.get("IsPaused")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        play_method,
        user: text(session.get("UserName")).map(str::to_owned),
        client: text(session.get("Client"))
            .or_else(|| text(session.get("DeviceName")))
            .map(str::to_owned),
        year: item.get("ProductionYear").and_then(Value::as_i64),
        ..event(MediaMode::Playing, source)
    }
}
pub fn normalize_jelly_latest(items: &Value, base_url: Option<&str>) -> MediaDisplayEvent {
    let Some(item) = items.as_array().and_then(|items| items.first()) else {
        return MediaDisplayEvent::idle("jellyfin");
    };
    let item_id = text(item.get("Id"));
    let title = text(item.get("Name")).or_else(|| text(item.get("SeriesName")));
    MediaDisplayEvent {
        title: Some(title.unwrap_or("New media").to_owned()),
        poster_url: item_poster(base_url, item_id),
        year: item.get("ProductionYear").and_then(Value::as_i64),
        media_type: text(item.get("Type")).map(str::to_owned),
        added_at: text(item.get("DateCreated")).map(str::to_owned),
        ..event(MediaMode::Landed, "jellyfin")
    }
}

pub fn normalize_silo_sessions(sessions: &Value, base_url: Option<&str>) -> MediaDisplayEvent {
    let Some(session) = sessions.as_array().and_then(|items| items.first()) else {
        return MediaDisplayEvent::idle("silo");
    };
    let duration = number(session.get("file_duration")).unwrap_or(0.0);
    let position = number(session.get("position_seconds")).unwrap_or(0.0);
    MediaDisplayEvent {
        title: Some(
            text(session.get("media_title"))
                .or_else(|| text(session.get("episode_name")))
                .unwrap_or("Playing")
                .to_owned(),
        ),
        poster_url: absolute_url(base_url, text(session.get("poster_url"))),
        progress_pct: (duration > 0.0).then(|| rounded_pct(position * 100.0 / duration)),
        remaining_minutes: remaining_minutes(duration, position, 1.0),
        paused: Some(
            session
                .get("is_paused")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        play_method: text(session.get("effective_play_method"))
            .or_else(|| text(session.get("play_method")))
            .or(Some("direct"))
            .map(str::to_owned),
        client: text(session.get("client_label"))
            .or_else(|| text(session.get("client_name")))
            .map(str::to_owned),
        quality: text(session.get("source_video_resolution"))
            .or_else(|| text(session.get("target_resolution")))
            .map(str::to_owned),
        video_codec: text(session.get("source_video_codec"))
            .or_else(|| text(session.get("target_video_codec")))
            .map(str::to_owned),
        audio_codec: text(session.get("source_audio_codec"))
            .or_else(|| text(session.get("target_audio_codec")))
            .map(str::to_owned),
        ..event(MediaMode::Playing, "silo")
    }
}

pub fn normalize_qbit_torrents(torrents: &Value) -> MediaDisplayEvent {
    let Some(torrent) = torrents.as_array().and_then(|items| {
        items
            .iter()
            .filter(|item| {
                text(item.get("state")).is_some_and(|state| ACTIVE_QBIT_STATES.contains(&state))
            })
            .max_by(|a, b| {
                let ap = number(a.get("progress")).unwrap_or(0.0);
                let bp = number(b.get("progress")).unwrap_or(0.0);
                ap.total_cmp(&bp).then_with(|| {
                    a.get("dlspeed")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .cmp(&b.get("dlspeed").and_then(Value::as_u64).unwrap_or(0))
                })
            })
    }) else {
        return MediaDisplayEvent::idle("qbittorrent");
    };
    let eta = torrent.get("eta").and_then(Value::as_u64);
    let eta_minutes = eta
        .filter(|eta| *eta < 8_640_000)
        .map(|eta| (eta.saturating_add(59) / 60).max(1));
    MediaDisplayEvent {
        title: Some(text(torrent.get("name")).unwrap_or("Download").to_owned()),
        progress_pct: Some(rounded_pct(
            number(torrent.get("progress")).unwrap_or(0.0) * 100.0,
        )),
        eta_minutes,
        speed_bytes_s: Some(torrent.get("dlspeed").and_then(Value::as_u64).unwrap_or(0)),
        download_id: text(torrent.get("hash")).map(str::to_owned),
        status: text(torrent.get("state")).map(str::to_owned),
        ..event(MediaMode::Incoming, "qbittorrent")
    }
}

fn duration_minutes(value: Option<&str>) -> Option<u64> {
    let mut text = value?.trim();
    let mut days = 0_u64;
    if let Some((day_text, rest)) = text.split_once('.') {
        days = day_text.parse().ok()?;
        text = rest;
    }
    let parts: Vec<_> = text.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let hours: u64 = parts[0].parse().ok()?;
    let minutes: u64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;
    let total = days * 86_400 + hours * 3600 + minutes * 60 + seconds.ceil() as u64;
    Some((total.saturating_add(59) / 60).max(1))
}
fn poster_from_images(images: Option<&Value>) -> Option<String> {
    images?.as_array()?.iter().find_map(|image| {
        let cover = text(image.get("coverType"))?;
        if !cover.eq_ignore_ascii_case("poster") {
            return None;
        }
        text(image.get("remoteUrl"))
            .or_else(|| text(image.get("url")))
            .map(str::to_owned)
    })
}

fn arr_progress(row: &Value) -> Option<f64> {
    let size = number(row.get("size")).unwrap_or(0.0);
    let left = number(row.get("sizeleft")).unwrap_or(0.0);
    (size > 0.0).then(|| rounded_pct((size - left).max(0.0) * 100.0 / size))
}

pub fn normalize_radarr_queue(payload: &Value) -> MediaDisplayEvent {
    let records = payload
        .get("records")
        .and_then(Value::as_array)
        .or_else(|| payload.as_array());
    let Some(row) = records.and_then(|items| items.first()) else {
        return MediaDisplayEvent::idle("radarr");
    };
    let movie = row.get("movie").unwrap_or(&Value::Null);
    MediaDisplayEvent {
        title: Some(
            text(movie.get("title"))
                .or_else(|| text(row.get("title")))
                .unwrap_or("Incoming")
                .to_owned(),
        ),
        poster_url: poster_from_images(movie.get("images")),
        progress_pct: arr_progress(row),
        eta_minutes: duration_minutes(text(row.get("timeleft"))),
        download_id: text(row.get("downloadId"))
            .or_else(|| text(row.get("downloadClientId")))
            .map(str::to_owned),
        status: text(row.get("status")).map(str::to_owned),
        ..event(MediaMode::Incoming, "radarr")
    }
}
pub fn normalize_sonarr_queue(payload: &Value) -> MediaDisplayEvent {
    let records = payload
        .get("records")
        .and_then(Value::as_array)
        .or_else(|| payload.as_array());
    let Some(row) = records.and_then(|items| items.first()) else {
        return MediaDisplayEvent::idle("sonarr");
    };
    let series = row.get("series").unwrap_or(&Value::Null);
    let episode = row.get("episode").unwrap_or(&Value::Null);
    let series_title = text(series.get("title")).unwrap_or("Series");
    let episode_title = text(episode.get("title"));
    let season = episode.get("seasonNumber").and_then(Value::as_u64);
    let number = episode.get("episodeNumber").and_then(Value::as_u64);
    let title = match (season, number, episode_title) {
        (Some(season), Some(number), Some(episode_title)) => {
            format!("{series_title} · S{season:02}E{number:02} · {episode_title}")
        }
        _ => text(row.get("title")).unwrap_or(series_title).to_owned(),
    };
    MediaDisplayEvent {
        title: Some(title),
        poster_url: poster_from_images(series.get("images")),
        progress_pct: arr_progress(row),
        eta_minutes: duration_minutes(text(row.get("timeleft"))),
        download_id: text(row.get("downloadId"))
            .or_else(|| text(row.get("downloadClientId")))
            .map(str::to_owned),
        status: text(row.get("status")).map(str::to_owned),
        media_type: Some("episode".into()),
        ..event(MediaMode::Incoming, "sonarr")
    }
}
fn priority(mode: &MediaMode) -> i8 {
    match mode {
        MediaMode::Playing => 40,
        MediaMode::Incoming => 30,
        MediaMode::Landed => 20,
        MediaMode::Idle => 0,
        MediaMode::Offline => -1,
    }
}

pub fn select_display_event(states: &[MediaDisplayEvent]) -> MediaDisplayEvent {
    states
        .iter()
        .max_by_key(|state| priority(&state.mode))
        .cloned()
        .unwrap_or_default()
}

pub fn fuse_media_states(mut states: Vec<MediaDisplayEvent>) -> MediaDisplayEvent {
    let qbit = states
        .iter()
        .find(|state| state.source.as_deref() == Some("qbittorrent"))
        .cloned();
    if let Some(qbit) = qbit
        && let Some(index) = states.iter().position(|state| {
            matches!(state.source.as_deref(), Some("radarr" | "sonarr"))
                && state.mode == MediaMode::Incoming
        })
    {
        states[index] = merge_incoming(&states[index], &qbit);
        states.retain(|state| state.source.as_deref() != Some("qbittorrent"));
    }
    select_display_event(&states)
}

pub fn merge_incoming(primary: &MediaDisplayEvent, qbit: &MediaDisplayEvent) -> MediaDisplayEvent {
    if primary.mode != MediaMode::Incoming {
        return qbit.clone();
    }
    if qbit.mode != MediaMode::Incoming {
        return primary.clone();
    }
    let mut out = primary.clone();
    let primary_id = primary
        .download_id
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let qbit_id = qbit
        .download_id
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let same_download = primary_id.is_empty() || qbit_id.is_empty() || primary_id == qbit_id;
    out.provider_chain = vec![
        provider_name(primary.source.as_deref().unwrap_or("arr")),
        provider_name("qbittorrent"),
    ];
    if same_download {
        if qbit.eta_minutes.is_some() {
            out.eta_minutes = qbit.eta_minutes;
        }
        if qbit.speed_bytes_s.is_some() {
            out.speed_bytes_s = qbit.speed_bytes_s;
        }
        if qbit.progress_pct.is_some() {
            out.progress_pct = qbit.progress_pct;
        }
    }
    out
}

pub fn offline(source: &str) -> MediaDisplayEvent {
    event(MediaMode::Offline, source)
}

fn same_origin(target: &str, provider_base: &str) -> bool {
    let Ok(target) = reqwest::Url::parse(target) else {
        return false;
    };
    let base = super::http::url(provider_base, "");
    let Ok(base) = reqwest::Url::parse(&base) else {
        return false;
    };
    target.scheme() == base.scheme()
        && target.host_str() == base.host_str()
        && target.port_or_known_default() == base.port_or_known_default()
}

pub async fn cache_live_poster(
    event: &mut MediaDisplayEvent,
    settings: &Settings,
    secrets: &ProviderSecrets,
    root: &Path,
) {
    let Some(remote_url) = event.poster_url.take() else {
        return;
    };
    event.poster_asset_id = None;
    let source = event.source.as_deref().unwrap_or_default();
    let verify_tls = settings
        .providers
        .get(source)
        .is_none_or(|provider| provider.verify_tls);
    let Ok(client) = HttpClient::new(verify_tls) else {
        return;
    };
    let provider_url = settings
        .providers
        .get(source)
        .map(|provider| provider.url.as_str())
        .unwrap_or_default();
    let same_origin = same_origin(&remote_url, provider_url);
    let authorization = secret(secrets, source, "api_key").map(|token| format!("Bearer {token}"));
    let headers = if same_origin {
        match source {
            "jellyfin" => vec![(
                "X-Emby-Token",
                secret(secrets, source, "api_key").unwrap_or(""),
            )],
            "silo" => authorization
                .as_deref()
                .map(|value| vec![("Authorization", value)])
                .unwrap_or_default(),
            "radarr" | "sonarr" => vec![(
                "X-Api-Key",
                secret(secrets, source, "api_key").unwrap_or(""),
            )],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let Ok(bytes) = client
        .get_binary(&remote_url, &headers, MAX_POSTER_BYTES)
        .await
    else {
        return;
    };
    cache_live_poster_bytes(event, root, &bytes);
}

pub fn cache_live_poster_bytes(event: &mut MediaDisplayEvent, root: &Path, bytes: &[u8]) {
    event.poster_url = None;
    event.poster_asset_id = None;
    if bytes.len() > MAX_POSTER_BYTES {
        return;
    }
    if aooscope_render::MediaStore::new(root)
        .and_then(|store| store.upsert(LIVE_POSTER_ASSET_ID, bytes, "live-poster"))
        .is_ok()
    {
        event.poster_asset_id = Some(LIVE_POSTER_ASSET_ID.into());
        event.poster_url = Some(format!("/api/media/{LIVE_POSTER_ASSET_ID}/file"));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("provider is not configured")]
    NotConfigured,
    #[error("provider request failed")]
    Request(#[from] HttpError),
    #[error("qBittorrent authentication failed")]
    Authentication,
}

fn configured<'a>(settings: &'a Settings, name: &str) -> Option<&'a ProviderSettings> {
    settings
        .providers
        .get(name)
        .filter(|provider| provider.enabled && !provider.url.trim().is_empty())
}

fn secret<'a>(secrets: &'a ProviderSecrets, provider: &str, key: &str) -> Option<&'a str> {
    secrets
        .get(provider)
        .and_then(|values| values.get(key))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

pub async fn collect_provider(
    name: &str,
    settings: &Settings,
    secrets: &ProviderSecrets,
) -> Result<MediaDisplayEvent, CollectorError> {
    let config = configured(settings, name).ok_or(CollectorError::NotConfigured)?;
    let client = HttpClient::new(config.verify_tls)?;
    let base = config.url.as_str();
    match name {
        "jellyfin" => {
            let sessions = client
                .get_json(
                    base,
                    "/Sessions",
                    &[(
                        "X-Emby-Token",
                        secret(secrets, name, "api_key").unwrap_or(""),
                    )],
                )
                .await?;
            let playing = normalize_jelly_sessions(&sessions, name, Some(base));
            if playing.mode == MediaMode::Playing {
                Ok(playing)
            } else {
                let latest = client
                    .get_json(
                        base,
                        "/Items/Latest",
                        &[transport_header(secrets, name, "api_key", "X-Emby-Token")],
                    )
                    .await?;
                Ok(normalize_jelly_latest(&latest, Some(base)))
            }
        }
        "silo" => {
            let token = secret(secrets, name, "api_key").unwrap_or("");
            let authorization = format!("Bearer {token}");
            let headers = [("Authorization", authorization.as_str())];
            match client
                .get_json(base, "/api/v1/profiles/household/sessions", &headers)
                .await
            {
                Ok(payload) => Ok(normalize_silo_sessions(&payload, Some(base))),
                Err(_) => {
                    let payload = client
                        .get_json(base, "/Sessions", &[("X-Emby-Token", token)])
                        .await?;
                    Ok(normalize_jelly_sessions(&payload, name, Some(base)))
                }
            }
        }
        "radarr" => collect_arr(&client, base, name, secrets).await,
        "sonarr" => collect_arr(&client, base, name, secrets).await,
        "qbittorrent" => {
            let username = secret(secrets, name, "username");
            let password = secret(secrets, name, "password");
            let cookie = match (username, password) {
                (Some(username), Some(password)) => {
                    Some(login_qbit(&client, base, username, password).await?)
                }
                (None, None) => None,
                _ => return Err(CollectorError::Authentication),
            };
            let headers = cookie
                .as_deref()
                .map(|cookie| vec![("Cookie", cookie)])
                .unwrap_or_default();
            let payload = client
                .get_json(base, "/api/v2/torrents/info", &headers)
                .await?;
            Ok(normalize_qbit_torrents(&payload))
        }
        _ => Err(CollectorError::NotConfigured),
    }
}

fn arr_queue_path(name: &str) -> &'static str {
    match name {
        "radarr" => "/api/v3/queue?page=1&pageSize=20&includeMovie=true",
        "sonarr" => "/api/v3/queue?page=1&pageSize=20&includeSeries=true&includeEpisode=true",
        _ => "/api/v3/queue?page=1&pageSize=20",
    }
}

async fn collect_arr(
    client: &HttpClient,
    base: &str,
    name: &str,
    secrets: &ProviderSecrets,
) -> Result<MediaDisplayEvent, CollectorError> {
    let header = transport_header(secrets, name, "api_key", "X-Api-Key");
    let payload = client
        .get_json(base, arr_queue_path(name), &[header])
        .await?;
    Ok(match name {
        "radarr" => normalize_radarr_queue(&payload),
        _ => normalize_sonarr_queue(&payload),
    })
}

fn transport_header<'a>(
    secrets: &'a ProviderSecrets,
    provider: &str,
    key: &str,
    header: &'a str,
) -> (&'a str, &'a str) {
    (header, secret(secrets, provider, key).unwrap_or(""))
}

async fn login_qbit(
    client: &HttpClient,
    base: &str,
    username: &str,
    password: &str,
) -> Result<String, CollectorError> {
    let response = client
        .post_form(
            base,
            "/api/v2/auth/login",
            &[("username", username), ("password", password)],
            &[("Referer", base)],
        )
        .await?;
    if !response.status().is_success() {
        return Err(CollectorError::Authentication);
    }
    let cookie = cookie_from_response(&response).ok_or(CollectorError::Authentication)?;
    let body = bounded_body(response, 256)
        .await
        .map_err(|_| CollectorError::Authentication)?;
    if body.len() > 256
        || !String::from_utf8_lossy(&body)
            .trim()
            .eq_ignore_ascii_case("ok.")
    {
        return Err(CollectorError::Authentication);
    }
    Ok(cookie)
}

fn provider_status(configured: bool, enabled: bool, online: bool, error: Option<String>) -> Value {
    let last_success = online.then(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or_default()
    });
    json!({
        "configured": configured,
        "enabled": enabled,
        "online": online,
        "last_success": last_success,
        "error": error
    })
}

pub async fn collect_media_state_with_status(
    settings: &Settings,
    secrets: &ProviderSecrets,
) -> (MediaDisplayEvent, BTreeMap<String, Value>) {
    let mut states = Vec::new();
    let mut statuses = BTreeMap::new();
    for provider in ["jellyfin", "silo", "radarr", "sonarr", "qbittorrent"] {
        let config = settings.providers.get(provider);
        let configured = config.is_some_and(|value| !value.url.trim().is_empty());
        let enabled = config.is_some_and(|value| value.enabled);
        if !configured || !enabled {
            statuses.insert(
                provider.to_owned(),
                provider_status(configured, enabled, false, None),
            );
            continue;
        }
        match collect_provider(provider, settings, secrets).await {
            Ok(state) => {
                statuses.insert(provider.to_owned(), provider_status(true, true, true, None));
                states.push(state);
            }
            Err(error) => {
                statuses.insert(
                    provider.to_owned(),
                    provider_status(true, true, false, Some(error.to_string())),
                );
                states.push(offline(provider));
            }
        }
    }
    (fuse_media_states(states), statuses)
}

pub async fn collect_media_state(
    settings: &Settings,
    secrets: &ProviderSecrets,
) -> MediaDisplayEvent {
    collect_media_state_with_status(settings, secrets).await.0
}

#[cfg(test)]
mod tests {
    use super::{arr_queue_path, duration_minutes, same_origin};

    #[test]
    fn poster_credentials_are_limited_to_provider_origin() {
        assert!(same_origin(
            "https://radarr.example.test/poster/1",
            "https://radarr.example.test"
        ));
        assert!(same_origin(
            "http://radarr.example.test:7878/poster/1",
            "radarr.example.test:7878"
        ));
        assert!(!same_origin(
            "https://image.tmdb.org/t/p/original/poster.jpg",
            "https://radarr.example.test"
        ));
        assert!(!same_origin(
            "https://radarr.example.test.evil.invalid/poster",
            "https://radarr.example.test"
        ));
    }

    #[test]
    fn duration_parser_handles_days_and_rounding() {
        assert_eq!(duration_minutes(Some("00:08:01")), Some(9));
        assert_eq!(duration_minutes(Some("1.01:00:00")), Some(1500));
        assert_eq!(duration_minutes(None), None);
    }

    #[test]
    fn arr_queue_paths_request_display_metadata() {
        assert!(arr_queue_path("radarr").contains("includeMovie=true"));
        assert!(arr_queue_path("sonarr").contains("includeSeries=true"));
        assert!(arr_queue_path("sonarr").contains("includeEpisode=true"));
    }

    #[tokio::test]
    async fn disabled_media_provider_is_configured_but_not_polled() {
        let settings: aooscope_types::Settings = serde_json::from_value(serde_json::json!({
            "providers": {"jellyfin": {"enabled": false, "url": "http://jellyfin.test"}}
        }))
        .unwrap();
        let (_, statuses) =
            super::collect_media_state_with_status(&settings, &Default::default()).await;
        let status = &statuses["jellyfin"];
        assert_eq!(status["configured"], true);
        assert_eq!(status["enabled"], false);
        assert_eq!(status["online"], false);
    }
}
