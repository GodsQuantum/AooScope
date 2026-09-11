#!/usr/bin/env python3
import json
import math
import os
import urllib.parse
import urllib.request
from pathlib import Path


def _idle(source=None):
    state = {"mode": "idle"}
    if source:
        state["source"] = source
    return state


def _absolute_url(base_url, value):
    if not value:
        return None
    if str(value).startswith(("http://", "https://")):
        return str(value)
    if not base_url:
        return str(value)
    return urllib.parse.urljoin(base_url.rstrip("/") + "/", str(value).lstrip("/"))


class JSONClient:
    def __init__(self, base_url, headers=None, timeout=3.0):
        self.base_url = base_url.rstrip("/")
        self.headers = dict(headers or {})
        self.timeout = float(timeout)

    def get(self, path, params=None):
        url = self.base_url + "/" + path.lstrip("/")
        if params:
            url += "?" + urllib.parse.urlencode(params)
        headers = {"Accept": "application/json", "User-Agent": "aooscope/0.1"}
        headers.update(self.headers)
        req = urllib.request.Request(url, headers=headers)
        with urllib.request.urlopen(req, timeout=self.timeout) as response:
            return json.load(response)


def normalize_jelly_sessions(sessions, source="jellyfin", base_url=None):
    active = [s for s in (sessions or []) if s.get("NowPlayingItem")]
    if not active:
        return _idle(source)
    session = active[0]
    item = session.get("NowPlayingItem") or {}
    play = session.get("PlayState") or {}
    runtime = item.get("RunTimeTicks") or 0
    position = play.get("PositionTicks") or 0
    progress = round(position * 100.0 / runtime, 1) if runtime else None
    method = str(play.get("PlayMethod") or "").lower()
    if session.get("TranscodingInfo"):
        method = "transcode"
    elif not method:
        method = "direct"
    poster_id = item.get("Id")
    poster_url = None
    if poster_id and base_url:
        poster_url = f"{base_url.rstrip('/')}/Items/{poster_id}/Images/Primary?maxWidth=360&quality=90"
    return {
        "mode": "playing",
        "source": source,
        "title": item.get("Name") or item.get("SeriesName") or "Playing",
        "year": item.get("ProductionYear"),
        "poster_id": poster_id,
        "poster_url": poster_url,
        "progress_pct": progress,
        "paused": bool(play.get("IsPaused")),
        "play_method": method,
        "user": session.get("UserName"),
        "client": session.get("Client") or session.get("DeviceName"),
    }


def normalize_silo_sessions(sessions, base_url=None):
    if not sessions:
        return _idle("silo")
    session = sessions[0]
    duration = float(session.get("file_duration") or 0)
    position = float(session.get("position_seconds") or 0)
    progress = round(position * 100.0 / duration, 1) if duration > 0 else None
    title = session.get("media_title") or session.get("episode_name") or "Playing"
    return {
        "mode": "playing",
        "source": "silo",
        "title": title,
        "poster_url": _absolute_url(base_url, session.get("poster_url")),
        "progress_pct": progress,
        "paused": bool(session.get("is_paused")),
        "play_method": session.get("effective_play_method") or session.get("play_method") or "direct",
        "client": session.get("client_label") or session.get("client_name"),
        "quality": session.get("source_video_resolution") or session.get("target_resolution"),
        "video_codec": session.get("source_video_codec") or session.get("target_video_codec"),
        "audio_codec": session.get("source_audio_codec") or session.get("target_audio_codec"),
    }


def normalize_jelly_latest(items, base_url=None):
    if not items:
        return _idle("jellyfin")
    item = items[0]
    item_id = item.get("Id")
    poster_url = None
    if item_id and base_url:
        poster_url = f"{base_url.rstrip('/')}/Items/{item_id}/Images/Primary?maxWidth=360&quality=90"
    return {
        "mode": "landed",
        "source": "jellyfin",
        "title": item.get("Name") or item.get("SeriesName") or "New media",
        "year": item.get("ProductionYear"),
        "media_type": item.get("Type"),
        "added_at": item.get("DateCreated"),
        "poster_id": item_id,
        "poster_url": poster_url,
    }

def normalize_qbit_torrents(torrents):
    active_states = {"downloading", "forcedDL", "stalledDL", "metaDL", "checkingDL", "allocating"}
    active = [t for t in (torrents or []) if t.get("state") in active_states]
    if not active:
        return _idle("qbittorrent")
    torrent = max(active, key=lambda t: (float(t.get("progress") or 0), int(t.get("dlspeed") or 0)))
    eta = torrent.get("eta")
    eta_minutes = None
    if isinstance(eta, (int, float)) and 0 <= eta < 8_640_000:
        eta_minutes = max(1, math.ceil(eta / 60))
    return {
        "mode": "incoming",
        "source": "qbittorrent",
        "title": torrent.get("name") or "Download",
        "progress_pct": round(float(torrent.get("progress") or 0) * 100, 1),
        "eta_minutes": eta_minutes,
        "speed_bytes_s": int(torrent.get("dlspeed") or 0),
        "download_id": torrent.get("hash"),
    }


def _duration_minutes(value):
    if not value:
        return None
    text = str(value).strip()
    days = 0
    if "." in text:
        day_text, text = text.split(".", 1)
        if day_text.isdigit():
            days = int(day_text)
    parts = text.split(":")
    if len(parts) != 3:
        return None
    try:
        hours, minutes, seconds = [int(float(x)) for x in parts]
    except ValueError:
        return None
    total = days * 86400 + hours * 3600 + minutes * 60 + seconds
    return max(1, math.ceil(total / 60))


def normalize_radarr_queue(payload):
    records = payload.get("records", []) if isinstance(payload, dict) else (payload or [])
    if not records:
        return _idle("radarr")
    row = records[0]
    movie = row.get("movie") or {}
    size = float(row.get("size") or 0)
    left = float(row.get("sizeleft") or 0)
    progress = round((size - left) * 100.0 / size, 1) if size else None
    poster = None
    for image in movie.get("images") or []:
        if str(image.get("coverType", "")).lower() == "poster":
            poster = image.get("remoteUrl") or image.get("url")
            break
    return {
        "mode": "incoming",
        "source": "radarr",
        "title": movie.get("title") or row.get("title") or "Incoming",
        "progress_pct": progress,
        "eta_minutes": _duration_minutes(row.get("timeleft")),
        "poster_url": poster,
        "download_id": row.get("downloadId") or row.get("downloadClientId"),
        "status": row.get("status"),
        "tracked_status": row.get("trackedDownloadStatus"),
    }


def select_display_event(*states):
    priority = {"playing": 40, "incoming": 30, "landed": 20, "idle": 0, "offline": -1}
    candidates = [s for s in states if s]
    if not candidates:
        return {"mode": "idle"}
    return max(candidates, key=lambda s: priority.get(s.get("mode"), 0))


def _safe_call(source, fn):
    try:
        return fn()
    except Exception:
        return {"mode": "offline", "source": source}


def merge_incoming(radarr_state, qbit_state):
    r = radarr_state or _idle("radarr")
    q = qbit_state or _idle("qbittorrent")
    if r.get("mode") != "incoming":
        return q
    if q.get("mode") != "incoming":
        return r
    out = dict(r)
    rid = str(r.get("download_id") or "").lower()
    qid = str(q.get("download_id") or "").lower()
    if not rid or not qid or rid == qid:
        if q.get("eta_minutes") is not None:
            out["eta_minutes"] = q["eta_minutes"]
        if q.get("speed_bytes_s") is not None:
            out["speed_bytes_s"] = q["speed_bytes_s"]
        if q.get("progress_pct") is not None:
            out["progress_pct"] = q["progress_pct"]
    return out

def collect_media_state(jelly_client=None, silo_client=None, radarr_client=None, qbit_client=None, now_unix=None):
    jelly_base = getattr(jelly_client, "base_url", None)
    silo_base = getattr(silo_client, "base_url", None)

    jelly_playing = _idle("jellyfin")
    latest = _idle("jellyfin")
    if jelly_client is not None:
        jelly_playing = _safe_call("jellyfin", lambda: normalize_jelly_sessions(
            jelly_client.get("/Sessions") or [], "jellyfin", jelly_base))
        latest = _safe_call("jellyfin", lambda: normalize_jelly_latest(
            jelly_client.get("/Items/Latest", {
                "Limit": 1, "IncludeItemTypes": "Movie,Episode", "GroupItems": "false"
            }) or [], jelly_base))

    silo_playing = _idle("silo")
    if silo_client is not None:
        silo_playing = _safe_call("silo", lambda: normalize_silo_sessions(
            silo_client.get("/api/v1/profiles/household/sessions") or [], silo_base))

    radarr = _idle("radarr")
    if radarr_client is not None:
        radarr = _safe_call("radarr", lambda: normalize_radarr_queue(
            radarr_client.get("/api/v3/queue", {"page": 1, "pageSize": 20, "includeMovie": "true"}) or {}))

    qbit = _idle("qbittorrent")
    if qbit_client is not None:
        qbit = _safe_call("qbittorrent", lambda: normalize_qbit_torrents(
            qbit_client.get("/api/v2/torrents/info", {"filter": "downloading"}) or []))

    playing = select_display_event(silo_playing, jelly_playing)
    incoming = merge_incoming(radarr, qbit)
    display = select_display_event(playing, incoming, latest)
    return {
        "playing": playing,
        "incoming": incoming,
        "latest": latest,
        "display": display,
        "sources": {
            "silo": silo_playing,
            "jellyfin": jelly_playing,
            "radarr": radarr,
            "qbittorrent": qbit,
        },
    }


def _read_secret(path):
    if not path:
        return None
    try:
        value = Path(path).read_text(encoding="utf-8").strip()
        return value or None
    except OSError:
        return None


def build_media_clients(env=None):
    env = os.environ if env is None else env
    clients = {}
    jelly_key = _read_secret(env.get("JELLYFIN_API_KEY_FILE"))
    if env.get("JELLYFIN_URL") and jelly_key:
        clients["jellyfin"] = JSONClient(env["JELLYFIN_URL"], {"X-Emby-Token": jelly_key})
    silo_key = _read_secret(env.get("SILO_API_KEY_FILE"))
    if env.get("SILO_URL") and silo_key:
        clients["silo"] = JSONClient(env["SILO_URL"], {"Authorization": f"Bearer {silo_key}"})
    radarr_key = _read_secret(env.get("RADARR_API_KEY_FILE"))
    if env.get("RADARR_URL") and radarr_key:
        clients["radarr"] = JSONClient(env["RADARR_URL"], {"X-Api-Key": radarr_key})
    if env.get("QBIT_URL"):
        clients["qbittorrent"] = JSONClient(env["QBIT_URL"])
    return clients
