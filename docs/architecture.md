# Architecture

AooScope is intentionally a **single-container control plane** for a small local LCD. The container owns data collection, display state, rendering supervision and the Admin UI.

## Runtime components

```text
Linux sysfs ───────────────┐
Proxmox / media APIs ──────┼──► telemetry.py ─► state.json + sensors/*.txt
optional providers ────────┘                         │
                                                     ▼
Admin UI ─► settings.json + private/providers.json ─► supervisor.py
                                                     │
                                                     ├─ backgrounds / poster cache
                                                     ├─ monitor.json
                                                     ▼
                                                  asterctl
                                                     │
                                                     ▼
                                                AOOSTAR LCD
```

### `aooscope.telemetry`

Collects cheap local sysfs metrics every few seconds, provider data on a bounded cadence, and slower storage/SMART state less frequently. External failures are isolated so one unavailable service cannot stop the state loop.

### `aooscope.supervisor`

Owns the `asterctl` process. It rebuilds the monitor profile only when display settings, brightness, active media identity or branding change. Progress updates stay in sensor text files and use `aoostar-rs` partial updates instead of restarting the display engine.

### `webui.py`

Same-origin Flask application for display and provider settings. Secrets are written to a separate private file and are never serialized back to the browser.

## Storage contract

Only `/app/cfg` is persistent. A normal installation contains:

```text
/app/cfg/
├── settings.json
├── private/providers.json
├── branding/
├── sensors/
├── cache/
├── aooscope/          # generated display assets
├── state.json
└── monitor.json
```

Generated files may be recreated. `settings.json`, `private/` and custom `branding/` are the important user-owned data.

## Security boundaries

AooScope does not need the Docker socket, host networking, `privileged: true`, or arbitrary host filesystem mounts. Hardware access is limited to the configured serial LCD device. Provider APIs should use read-only accounts/tokens wherever possible.


## Page designer pipeline

`pages.json` is the normalized source of carousel drafts. `media.json` + `media/` hold reusable assets. `page_compiler.py` converts an applied document into `compiled/<revision>/monitor.json` and assets. `current.json` / `previous.json` provide atomic promotion and rollback. The display supervisor owns `asterctl`; drafts never touch the LCD until Apply. `animation_runtime.py` writes bounded synthetic phase sensors for native partial-update animations.
