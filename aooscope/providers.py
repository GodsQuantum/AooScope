#!/usr/bin/env python3
import json
import os
import ssl
import urllib.parse
import urllib.request


def normalize_url(value):
    value = str(value or "").strip().rstrip("/")
    if value and "://" not in value:
        value = "http://" + value
    return value


def _context(verify_tls=True, ca_file=None):
    if not verify_tls:
        return ssl._create_unverified_context()
    if ca_file:
        return ssl.create_default_context(cafile=ca_file)
    return None


def _open(req, timeout, verify_tls=True, ca_file=None):
    return urllib.request.urlopen(req, timeout=timeout, context=_context(verify_tls, ca_file))


def _json_request(url, headers=None, data=None, timeout=4.0, verify_tls=True, ca_file=None):
    req = urllib.request.Request(url, data=data, headers={"User-Agent": "aooscope/0.1", **(headers or {})})
    with _open(req, timeout, verify_tls, ca_file) as response:
        raw = response.read(2_000_000)
        if not raw:
            return {}
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return {"text": raw.decode("utf-8", "replace")[:200]}


def _proxmox(base, cfg, secrets, timeout):
    token = str(secrets.get("api_token") or "").strip()
    if not token:
        raise ValueError("API token is required")
    header = token if token.startswith("PVEAPIToken=") else "PVEAPIToken=" + token
    data = _json_request(
        base + "/api2/json/version", {"Authorization": header}, timeout=timeout,
        verify_tls=cfg.get("verify_tls", True), ca_file=os.getenv("PVE_CA_FILE") or None,
    )
    return {"ok": True, "message": "Connected to Proxmox", "version": ((data.get("data") or {}).get("version"))}


def _beszel(base, cfg, secrets, timeout):
    email, password = secrets.get("email"), secrets.get("password")
    if not email or not password:
        raise ValueError("Email and password are required")
    payload = json.dumps({"identity": email, "password": password}).encode()
    auth = _json_request(base + "/api/collections/users/auth-with-password", {"Content-Type": "application/json"}, payload, timeout, cfg.get("verify_tls", True))
    token = auth.get("token")
    if not token:
        raise ValueError("Beszel authentication failed")
    _json_request(base + "/api/collections/systems/records?perPage=1", {"Authorization": "Bearer " + token}, timeout=timeout, verify_tls=cfg.get("verify_tls", True))
    return {"ok": True, "message": "Connected to Beszel"}


def _simple_get(base, path, cfg, headers, timeout, message):
    _json_request(base + path, headers, timeout=timeout, verify_tls=cfg.get("verify_tls", True))
    return {"ok": True, "message": message}


def _qbittorrent(base, cfg, secrets, timeout):
    verify = cfg.get("verify_tls", True)
    username, password = secrets.get("username"), secrets.get("password")
    if username or password:
        cookies = urllib.request.HTTPCookieProcessor()
        handlers = [cookies]
        if base.startswith("https://") and not verify:
            handlers.append(urllib.request.HTTPSHandler(context=_context(False)))
        opener = urllib.request.build_opener(*handlers)
        body = urllib.parse.urlencode({"username": username or "", "password": password or ""}).encode()
        req = urllib.request.Request(base + "/api/v2/auth/login", data=body, headers={"Content-Type": "application/x-www-form-urlencoded", "Referer": base, "User-Agent": "aooscope/0.1"})
        with opener.open(req, timeout=timeout) as response:
            if response.read(64).decode("utf-8", "replace").strip().lower() not in ("ok.", "ok"):
                raise ValueError("qBittorrent authentication failed")
        with opener.open(base + "/api/v2/app/version", timeout=timeout) as response:
            version = response.read(128).decode("utf-8", "replace").strip()
    else:
        req = urllib.request.Request(base + "/api/v2/app/version", headers={"User-Agent": "aooscope/0.1"})
        with _open(req, timeout, verify) as response:
            version = response.read(128).decode("utf-8", "replace").strip()
    return {"ok": True, "message": "Connected to qBittorrent", "version": version}


def test_provider(name, cfg, secrets=None, timeout=4.0):
    cfg, secrets = dict(cfg or {}), dict(secrets or {})
    base = normalize_url(cfg.get("url"))
    if not base:
        return {"ok": False, "message": "URL / IP:port is required"}
    try:
        if name == "proxmox": return _proxmox(base, cfg, secrets, timeout)
        if name == "beszel": return _beszel(base, cfg, secrets, timeout)
        if name == "jellyfin": return _simple_get(base, "/Sessions", cfg, {"X-Emby-Token": secrets.get("api_key", "")}, timeout, "Connected to Jellyfin")
        if name == "silo": return _simple_get(base, "/api/v1/profiles/household/sessions", cfg, {"Authorization": "Bearer " + secrets.get("api_key", "")}, timeout, "Connected to Silo")
        if name in ("radarr", "sonarr"): return _simple_get(base, "/api/v3/system/status", cfg, {"X-Api-Key": secrets.get("api_key", "")}, timeout, f"Connected to {name.title()}")
        if name == "qbittorrent": return _qbittorrent(base, cfg, secrets, timeout)
        if name == "ollama": return _simple_get(base, "/api/version", cfg, {}, timeout, "Connected to Ollama")
        if name == "immich": return _simple_get(base, "/", cfg, {"x-api-key": secrets.get("api_key", "")}, timeout, "Connected to Immich")
        return {"ok": False, "message": "Unsupported provider"}
    except Exception as exc:
        return {"ok": False, "message": f"{type(exc).__name__}: {exc}"}
