<p align="center">
  <img src="docs/assets/logo.svg" width="170" alt="AooScope logo">
</p>

<h1 align="center">AooScope</h1>

<p align="center"><strong>A glanceable smart dashboard for AOOSTAR LCD systems.</strong></p>

<p align="center">
  <img src="https://img.shields.io/badge/display-960×376-18d7ff" alt="960x376 display">
  <img src="https://img.shields.io/badge/runtime-Docker-2496ed" alt="Docker">
  <img src="https://img.shields.io/badge/license-MIT-3dd7cf" alt="MIT license">
  <img src="https://img.shields.io/badge/image-GHCR-54cd8a" alt="GHCR image">
</p>

<p align="center">🇫🇷 <a href="README.fr.md">README en français</a></p>

---

AooScope turns the small AOOSTAR LCD into a useful server display rather than a tiny wall of text. It combines large values with visual gauges, storage health, compute telemetry and event-driven media cards that can be understood from across the room.

It is a fork and substantial rewrite of `xavtb78/aoostar-proxmox-lcd`, while keeping `zehnm/aoostar-rs` as the low-level display engine.

## ✨ Highlights

- **Far-glance UI** — large numerals, semantic colour, visual gauges and protected text zones designed for a 960×376 panel.
- **Hardware telemetry** — Linux sysfs temperatures, Radeon activity and shared GPU memory without a host agent.
- **Proxmox provider** — CPU, RAM, guests, storage, ZFS and SMART through the native read-only API.
- **Media-aware display** — Jellyfin/Silo playback and Radarr/qBittorrent arrivals can temporarily pre-empt the normal carousel.
- **Poster-first media cards** — `PLAYING 42%`, `READY IN 9 MIN` and `JUST LANDED` instead of dense tables.
- **Admin UI** — configure display behaviour and provider `IP:port`/URLs from the browser.
- **Provider secrets stay private** — saved separately with mode `0600` and never returned by the settings API.
- **Software brightness schedule** — choose a base luminance and schedules such as `22:00–08:00 → 70%`.
- **Failure isolation** — an offline provider does not stop the LCD.
- **Pull-and-run container** — no Rust/Python toolchain required on the target system.

> **Brightness note:** no documented native WTR MAX backlight command is currently exposed by `aoostar-rs`. AooScope brightness is therefore software luminance: it scales the rendered pixels, not the physical backlight power.

## 🧩 Visual Page Designer

AooScope 0.2 adds a 960×376 WYSIWYG editor in the Admin UI. Create, duplicate, reorder, enable/disable and delete carousel pages; drag live sensors onto the canvas; choose value, bar, gauge, ring, badge or sparkline widgets; upload reusable media; preview drafts; and apply atomically to the LCD with rollback.

Images can be reused across pages. **Animate** turns a logo into an orbital HTML/CSS preview; on the WTR MAX the orbit is driven by a synthetic sensor and `aoostar-rs` partial updates (default 5 FPS, capped at 8 FPS) instead of full-frame video. Uploaded GIF/video files are accepted as animation sources, but full-frame high-FPS playback is intentionally not used on the serial LCD.

Edits are drafts until **Apply to LCD**. Brightness schedules re-render the last applied revision without promoting unpublished drafts.

## 🚀 Quick start

Requirements:

- Linux + Docker Engine / compatible Compose;
- a supported AOOSTAR LCD exposed as a serial device, commonly `/dev/ttyACM0`;
- permission for Docker to open that device.

```bash
mkdir -p aooscope/data
cd aooscope
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/compose.yaml
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/.env.example
cp .env.example .env

docker compose up -d
```

Open `http://127.0.0.1:8765` locally, or change `AOOSCOPE_BIND_ADDRESS` to a trusted LAN address.

The image is published as:

```text
ghcr.io/godsquantum/aooscope:latest
```

## 🖥️ Display behaviour

The normal carousel is intentionally small:

```text
Splash → Home → Storage → Compute
```

When a media event is present, Media becomes the first page. When the event disappears, AooScope returns to the normal carousel automatically.

Text is kept visually above dynamic gauges by reserving transparent windows inside gauge assets, so the value/label cannot be covered at high utilisation.

## 🔌 Providers

The Admin UI currently understands configuration for:

- Proxmox VE
- Beszel
- Jellyfin
- Silo
- Radarr
- Sonarr
- qBittorrent
- Immich
- Ollama

Proxmox, Jellyfin, Silo, Radarr and qBittorrent already feed runtime display state. Other providers are available for connection testing/configuration and are designed to be added to new pages without changing the display engine.

See [`docs/providers.md`](docs/providers.md).

## 🧱 Architecture

```text
Provider APIs + Linux sysfs
          │
          ▼
   normalized state
          │
    telemetry loop
          │
          ├── sensors/*.txt ─────┐
          └── state.json         │
                                 ▼
Admin UI ── settings.json ─► display supervisor ─► asterctl ─► AOOSTAR LCD
              │                   ▲
              └─ private secrets ─┘
```

AooScope stays in one container. It does **not** require host networking, the Docker socket, privileged mode or a monitoring daemon installed on the Proxmox host.

See [`docs/architecture.md`](docs/architecture.md).

## 🔐 Security

The Admin UI has no built-in login. The example Compose binds to `127.0.0.1` by default. Use an authenticated reverse proxy or VPN for remote access and do not expose port `8765` directly to the internet.

See [`SECURITY.md`](SECURITY.md).

## 🛠️ Development

```bash
python -m venv .venv
. .venv/bin/activate
pip install flask pillow
python -m unittest discover -s tests -p 'test_aooscope_*.py' -v
bash tests/test_deployment.sh
```

Local container build:

```bash
docker compose -f compose.yaml -f compose.dev.yaml up -d --build
```

## 🙏 Credits

- [`xavtb78/aoostar-proxmox-lcd`](https://github.com/xavtb78/aoostar-proxmox-lcd) — project this fork started from.
- [`zehnm/aoostar-rs`](https://github.com/zehnm/aoostar-rs) — reverse-engineered AOOSTAR display protocol and `asterctl`/`aster-sysinfo`.

AooScope is independent and is not affiliated with or endorsed by AOOSTAR. See [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
