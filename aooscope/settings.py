#!/usr/bin/env python3
import copy
import json
import os
import tempfile
from datetime import datetime
from zoneinfo import ZoneInfo
from pathlib import Path

PROVIDER_DEFAULTS = {
    "proxmox": {"enabled": False, "url": "", "node": "pve", "verify_tls": True},
    "beszel": {"enabled": False, "url": "", "system": "", "verify_tls": True},
    "jellyfin": {"enabled": False, "url": "", "verify_tls": True},
    "silo": {"enabled": False, "url": "", "verify_tls": True},
    "radarr": {"enabled": False, "url": "", "verify_tls": True},
    "sonarr": {"enabled": False, "url": "", "verify_tls": True},
    "qbittorrent": {"enabled": False, "url": "", "verify_tls": True},
    "immich": {"enabled": False, "url": "", "verify_tls": True},
    "ollama": {"enabled": False, "url": "", "verify_tls": True},
}

DEFAULT_SETTINGS = {
    "display": {
        "brand": "AOOSCOPE",
        "brightness": 100,
        "schedule_enabled": False,
        "timezone": "UTC",
        "schedule": [{"start": "22:00", "end": "08:00", "brightness": 70}],
        "switch_seconds": 8,
    },
    "providers": PROVIDER_DEFAULTS,
}

SECRET_FIELDS = {
    "proxmox": ("api_token",),
    "beszel": ("email", "password"),
    "jellyfin": ("api_key",),
    "silo": ("api_key",),
    "radarr": ("api_key",),
    "sonarr": ("api_key",),
    "qbittorrent": ("username", "password"),
    "immich": ("api_key",),
    "ollama": (),
}

def _deep_merge(base, override):
    out = copy.deepcopy(base)
    if not isinstance(override, dict):
        return out
    for key, value in override.items():
        if isinstance(value, dict) and isinstance(out.get(key), dict):
            out[key] = _deep_merge(out[key], value)
        else:
            out[key] = copy.deepcopy(value)
    return out


def _atomic_json(path, data, mode=0o644):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass


def load_settings(path=None):
    if not path:
        return copy.deepcopy(DEFAULT_SETTINGS)
    try:
        raw = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        raw = {}
    settings = _deep_merge(DEFAULT_SETTINGS, raw)
    settings["display"]["brightness"] = max(0, min(100, int(settings["display"].get("brightness", 100))))
    settings["display"]["switch_seconds"] = max(2, min(120, int(settings["display"].get("switch_seconds", 8))))
    return settings


def load_secrets(path=None):
    if not path:
        return {}
    try:
        raw = json.loads(Path(path).read_text(encoding="utf-8"))
        return raw if isinstance(raw, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}

def _time_minutes(value):
    try:
        hour, minute = str(value).split(":", 1)
        hour = int(hour); minute = int(minute)
        if not (0 <= hour < 24 and 0 <= minute < 60):
            raise ValueError
        return hour * 60 + minute
    except (ValueError, TypeError):
        return None


def effective_brightness(settings, now=None):
    display = (settings or {}).get("display") or {}
    base = max(0, min(100, int(display.get("brightness", 100))))
    if not display.get("schedule_enabled"):
        return base
    tz_name = str(display.get("timezone") or "UTC").strip() or "UTC"
    try:
        zone = ZoneInfo(tz_name)
    except Exception:
        zone = ZoneInfo("UTC")
    if now is None:
        now = datetime.now(zone)
    elif now.tzinfo is not None:
        now = now.astimezone(zone)
    current = now.hour * 60 + now.minute
    for rule in display.get("schedule") or []:
        start = _time_minutes(rule.get("start"))
        end = _time_minutes(rule.get("end"))
        if start is None or end is None:
            continue
        active = (start <= current < end) if start < end else (current >= start or current < end)
        if active:
            try:
                return max(0, min(100, int(rule.get("brightness", base))))
            except (ValueError, TypeError):
                return base
    return base


def public_settings(settings, secrets=None):
    out = copy.deepcopy(settings)
    secrets = secrets or {}
    for name, provider in out.get("providers", {}).items():
        provider["secret_set"] = any(bool(secrets.get(name, {}).get(field)) for field in SECRET_FIELDS.get(name, ()))
        for field in SECRET_FIELDS.get(name, ()):
            provider.pop(field, None)
    return out

def save_settings(payload, public_path, private_path):
    current = load_settings(public_path)
    incoming = payload if isinstance(payload, dict) else {}
    merged = _deep_merge(current, incoming)
    secrets = load_secrets(private_path)

    for name, provider in (incoming.get("providers") or {}).items():
        if name not in PROVIDER_DEFAULTS or not isinstance(provider, dict):
            continue
        bucket = dict(secrets.get(name) or {})
        for field in SECRET_FIELDS.get(name, ()):
            value = provider.get(field)
            if value is not None and str(value).strip():
                bucket[field] = str(value).strip()
            if provider.get(f"clear_{field}") is True:
                bucket.pop(field, None)
            merged["providers"][name].pop(field, None)
            merged["providers"][name].pop(f"clear_{field}", None)
        if bucket:
            secrets[name] = bucket
        else:
            secrets.pop(name, None)

    for name in list(merged.get("providers", {})):
        if name not in PROVIDER_DEFAULTS:
            merged["providers"].pop(name, None)
    merged = load_settings_from_mapping(merged)
    _atomic_json(public_path, merged, 0o644)
    _atomic_json(private_path, secrets, 0o600)
    return merged


def load_settings_from_mapping(raw):
    settings = _deep_merge(DEFAULT_SETTINGS, raw or {})
    display = settings["display"]
    try:
        display["brightness"] = max(0, min(100, int(display.get("brightness", 100))))
    except (ValueError, TypeError):
        display["brightness"] = 100
    try:
        display["switch_seconds"] = max(2, min(120, int(display.get("switch_seconds", 8))))
    except (ValueError, TypeError):
        display["switch_seconds"] = 8
    display["brand"] = str(display.get("brand") or "AOOSCOPE").strip()[:32] or "AOOSCOPE"
    display["timezone"] = str(display.get("timezone") or "UTC").strip()[:64] or "UTC"
    try:
        ZoneInfo(display["timezone"])
    except Exception:
        display["timezone"] = "UTC"
    return settings
