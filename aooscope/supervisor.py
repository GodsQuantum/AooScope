#!/usr/bin/env python3
import hashlib
import json
import os
import signal
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

from PIL import Image, ImageOps, ImageDraw, ImageEnhance

from aooscope.panels import build_monitor_config, generate_backgrounds, generate_splash
from aooscope.settings import load_settings, load_secrets, effective_brightness

ACTIVE_MODES = {"playing", "incoming", "landed"}


def active_media_event(state):
    event = ((state or {}).get("media") or {}).get("display") or {}
    return event if event.get("mode") in ACTIVE_MODES else None


def event_signature(event):
    if not event:
        return None
    return tuple(str(event.get(k) or "") for k in ("mode", "source", "title", "poster_url"))


def profile_signature(event, brightness, brand, switch_seconds, splash_image):
    return (event_signature(event), int(brightness), str(brand), float(switch_seconds), str(splash_image or ""))


def _short_title(value, limit=28):
    value = " ".join(str(value or "MEDIA").split())
    return value if len(value) <= limit else value[: limit - 1].rstrip() + "…"


def _headline(event):
    mode = (event or {}).get("mode")
    if mode == "incoming":
        eta = event.get("eta_minutes")
        return f"READY IN {int(eta)} MIN" if eta else "INCOMING"
    if mode == "playing":
        p = event.get("progress_pct")
        return f"PLAYING {int(round(float(p)))}%" if p is not None else "PLAYING"
    if mode == "landed":
        return "JUST LANDED"
    return "MEDIA"


def media_sensor_lines(event):
    event = event or {}
    progress = event.get("progress_pct")
    eta = event.get("eta_minutes")
    lines = [
        f"aooscope_media_display_headline: {_headline(event)}",
        f"aooscope_media_display_title_short: {_short_title(event.get('title'))}",
        f"aooscope_media_display_progress_pct: {0 if progress is None else round(float(progress), 1)}",
        f"aooscope_media_display_eta_minutes: {0 if eta is None else int(eta)}",
    ]
    return "\n".join(lines) + "\n"


def desired_monitor_config(event, splash_image=None, media_image="aooscope/media.jpg", switch_seconds=8, brightness=100):
    cfg = build_monitor_config(
        media_active=bool(event), media_image=media_image,
        switch_seconds=switch_seconds, splash_image=splash_image, brightness=brightness,
    )
    if event:
        panels = cfg.get("diy", [])
        media = [p for p in panels if p.get("id") == "media"]
        rest = [p for p in panels if p.get("id") != "media"]
        cfg["diy"] = media + rest
        cfg["mianban"] = list(range(1, len(cfg["diy"]) + 1))
    return cfg


def _atomic_text(path, text):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(text)
            f.flush(); os.fsync(f.fileno())
        os.replace(tmp, path)
    finally:
        try: os.unlink(tmp)
        except OSError: pass


def _atomic_json(path, payload):
    _atomic_text(path, json.dumps(payload, indent=2, ensure_ascii=False) + "\n")


def _read_state(path):
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError):
        return {}


def _read_secret(path):
    if not path:
        return None
    try:
        return Path(path).read_text(encoding="utf-8").strip() or None
    except OSError:
        return None


def _poster_headers(source, secrets=None):
    secrets = secrets or {}
    if source == "jellyfin":
        token = (secrets.get("jellyfin") or {}).get("api_key") or _read_secret(os.getenv("JELLYFIN_API_KEY_FILE"))
        return {"X-Emby-Token": token} if token else {}
    if source == "silo":
        token = (secrets.get("silo") or {}).get("api_key") or _read_secret(os.getenv("SILO_API_KEY_FILE"))
        return {"Authorization": f"Bearer {token}"} if token else {}
    return {}


def _download_poster(event, cache_dir, secrets=None):
    url = (event or {}).get("poster_url")
    if not url:
        return None
    digest = hashlib.sha256(url.encode()).hexdigest()[:16]
    path = Path(cache_dir) / f"poster-{digest}.jpg"
    if path.exists() and path.stat().st_size > 512:
        return path
    req = urllib.request.Request(url, headers={"User-Agent":"aooscope/0.1", **_poster_headers(event.get("source"), secrets)})
    try:
        with urllib.request.urlopen(req, timeout=4) as r:
            ctype = (r.headers.get("Content-Type") or "").lower()
            if "image" not in ctype:
                return None
            data = r.read(5_000_001)
        if len(data) > 5_000_000:
            return None
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        with Image.open(path) as im:
            im.verify()
        return path
    except Exception:
        try: path.unlink()
        except OSError: pass
        return None


def _media_background(config_dir, event, brightness=100, secrets=None):
    root = Path(config_dir)
    base = root / "aooscope" / "media.jpg"
    poster = _download_poster(event, root / "cache" / "posters", secrets=secrets)
    if not poster or not base.exists():
        return "aooscope/media.jpg"
    digest = hashlib.sha256((str(poster)+str(event_signature(event))+str(int(brightness))).encode()).hexdigest()[:12]
    out = root / "aooscope" / f"media-event-{digest}.jpg"
    if out.exists():
        return f"aooscope/{out.name}"
    with Image.open(base) as bg, Image.open(poster) as src:
        canvas = bg.convert("RGB")
        fitted = ImageOps.fit(src.convert("RGB"), (264, 340), method=Image.Resampling.LANCZOS)
        mask = Image.new("L", fitted.size, 0)
        ImageDraw.Draw(mask).rounded_rectangle((0,0,263,339), radius=24, fill=255)
        canvas.paste(fitted, (22,18), mask)
        if int(brightness) != 100:
            canvas = ImageEnhance.Brightness(canvas).enhance(max(0, min(100, int(brightness))) / 100.0)
        canvas.save(out, "JPEG", quality=95, optimize=True)
    return f"aooscope/{out.name}"


def _brightness_asset(config_dir, relative_path, brightness):
    if not relative_path or int(brightness) >= 100:
        return relative_path
    root = Path(config_dir)
    source = Path(relative_path)
    source = source if source.is_absolute() else root / source
    if not source.is_file():
        return relative_path
    digest = hashlib.sha256((str(source)+str(source.stat().st_mtime_ns)+str(int(brightness))).encode()).hexdigest()[:12]
    out = root / "cache" / "brightness" / f"asset-{digest}.jpg"
    out.parent.mkdir(parents=True, exist_ok=True)
    if not out.exists():
        with Image.open(source) as im:
            dimmed = ImageEnhance.Brightness(im.convert("RGB")).enhance(max(0, min(100, int(brightness))) / 100.0)
            dimmed.save(out, "JPEG", quality=95, optimize=True)
    try:
        return str(out.relative_to(root))
    except ValueError:
        return str(out)


class DisplaySupervisor:
    def __init__(self):
        self.config_dir = Path(os.getenv("AOOSCOPE_CONFIG_DIR_IN_CONTAINER", "/app/cfg"))
        self.state_path = Path(os.getenv("AOOSCOPE_STATE_PATH", str(self.config_dir / "state.json")))
        self.monitor_path = self.config_dir / os.getenv("AOOSCOPE_MONITOR_CONFIG", "monitor.json")
        self.media_sensor_path = self.config_dir / "sensors" / "media-display.txt"
        self.splash = os.getenv("AOOSCOPE_SPLASH_IMAGE") or None
        self.brand = os.getenv("AOOSCOPE_BRAND", "AOOSCOPE")
        self.switch_seconds = float(os.getenv("AOOSCOPE_SWITCH_SECONDS", "8"))
        self.settings_path = Path(os.getenv("AOOSCOPE_SETTINGS_PATH", str(self.config_dir / "settings.json")))
        self.private_path = Path(os.getenv("AOOSCOPE_PRIVATE_SETTINGS_PATH", str(self.config_dir / "private" / "providers.json")))
        self.device = os.getenv("AOOSCOPE_DEVICE", "/dev/ttyACM0")
        self.proc = None
        self.signature = object()
        self.running = True

    def load_promoted_revision(self):
        pointer = self.config_dir / "compiled" / "current.json"
        try:
            data = json.loads(pointer.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return None
        rid = str(data.get("revision_id") or "")
        config_dir = self.config_dir / "compiled" / rid
        if not rid or not (config_dir / "monitor.json").is_file():
            return None
        return {**data, "config_dir": str(config_dir), "monitor_path": str(config_dir / "monitor.json")}

    def current_display_profile(self):
        if self.settings_path.is_file():
            settings = load_settings(self.settings_path)
            display = settings.get("display") or {}
            return {
                "brand": display.get("brand") or self.brand,
                "brightness": effective_brightness(settings),
                "switch_seconds": float(display.get("switch_seconds") or self.switch_seconds),
                "splash_image": self.splash,
            }
        return {
            "brand": self.brand,
            "brightness": max(0, min(100, int(os.getenv("AOOSCOPE_BRIGHTNESS", "100")))),
            "switch_seconds": self.switch_seconds,
            "splash_image": self.splash,
        }

    def _cmd(self):
        return [
            "asterctl", "--device", self.device,
            "--config-dir", str(self.config_dir),
            "--config", self.monitor_path.name,
            "--font-dir", "/app/fonts",
            "--sensor-path", str(self.config_dir / "sensors"),
            "--sensor-mapping", str(self.config_dir / "sensor-mapping.cfg"),
        ]

    def stop_display(self):
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()
            try: self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.proc.kill(); self.proc.wait(timeout=2)
        self.proc = None

    def start_display(self):
        if not Path(self.device).exists():
            print(f"display device not found: {self.device}", flush=True)
            return
        self.proc = subprocess.Popen(self._cmd())
        print(f"display engine pid={self.proc.pid}", flush=True)

    def apply(self, event, force=False):
        _atomic_text(self.media_sensor_path, media_sensor_lines(event))
        profile = self.current_display_profile()
        brightness = profile["brightness"]
        sig = profile_signature(
            event, brightness, profile["brand"], profile["switch_seconds"], profile["splash_image"]
        )
        if not force and sig == self.signature:
            return
        generate_backgrounds(self.config_dir, brand=profile["brand"], brightness=brightness)
        secrets = load_secrets(self.private_path)
        image = _media_background(self.config_dir, event, brightness, secrets) if event else "aooscope/media.jpg"
        if profile["splash_image"]:
            splash = _brightness_asset(self.config_dir, profile["splash_image"], brightness)
        else:
            splash_path = generate_splash(self.config_dir, brand=profile["brand"], brightness=brightness)
            splash = str(splash_path.relative_to(self.config_dir))
        cfg = desired_monitor_config(
            event, splash, image, profile["switch_seconds"], brightness=brightness
        )
        _atomic_json(self.monitor_path, cfg)
        self.signature = sig
        self.stop_display(); self.start_display()
        print(
            f"display profile={'media' if event else 'normal'} brightness={brightness} signature={sig}",
            flush=True,
        )

    def loop(self):
        self.apply(active_media_event(_read_state(self.state_path)), force=True)
        while self.running:
            event = active_media_event(_read_state(self.state_path))
            self.apply(event)
            if not self.proc or self.proc.poll() is not None:
                print("display engine exited; restarting", flush=True)
                self.start_display()
            time.sleep(2)


def main():
    supervisor = DisplaySupervisor()

    def _stop(*_):
        supervisor.running = False
        supervisor.stop_display()

    signal.signal(signal.SIGTERM, _stop)
    signal.signal(signal.SIGINT, _stop)
    try:
        supervisor.loop()
    finally:
        supervisor.stop_display()


if __name__ == "__main__":
    main()
