# AooScope Visual + Provider Restoration Design

## Goal
Restore and improve the pre-cutover visual quality of both the web UI and the 960×376 LCD experience while keeping the production architecture Rust + Svelte only.

The public repository must ship the attractive UI and LCD templates by default. Public branding is AooScope only; no private instance names, LAN addresses, node names, credentials, may be hardcoded.

## Visual reference
The pre-cutover frontend and LCD assets are the baseline reference, not the simplified post-cutover UI.

Reference web language:
- dark navy radial background;
- cyan/green/amber accents;
- rounded cards with subtle borders;
- top header + status pill;
- tabs: Pages, Media, Display, Providers;
- 960×376 visual designer with Carousel sidebar, sensor/widget/media palette, inspector, preview and Apply controls.

Reference LCD language:
- dark near-black/navy base;
- restrained cyan accent bar;
- large readable values;
- graphical gauges and storage cards;
- minimal text sized for the physical panel;
- high contrast and low clutter.

## Public LCD templates
The public repository ships six official templates:

1. **Splash** — AooScope logo/brand treatment; optional Orbit animation.
2. **Home** — CPU, RAM, CPU temperature, plus compact guest/runtime status.
3. **Storage** — six SATA bays with disk name, temperature, thermal visualization and SMART health.
4. **Storage M.2** — optional/disabled-by-default page for NVMe/M.2 devices, cache/scratch/pool roles, temperature and health.
5. **Compute** — GPU load, CPU load, shared/GTT memory, GPU temperature and CPU temperature.
6. **Media** — poster/art at left and media status card at right with title, provider/source, progress, ETA and transfer rate when meaningful.

All pages are ordinary editable pages: enable/disable, reorder, duplicate and customize. Only Storage M.2 is disabled by default; the five baseline pages remain the default active carousel.

The renderer must reproduce the visual richness of the old `home.jpg`, `storage.jpg`, `compute.jpg`, `media.jpg` layouts without requiring private branding. Decorative elements may be generated in Rust or bundled as generic AooScope assets, but live values remain real editable widgets.

## Web UI
The Svelte frontend keeps the old information architecture and restores its visual quality.

Pages must again expose:
- Carousel cards with enabled state, duration, revision, drag reorder, duplicate, restore and delete;
- explicit Save carousel and Apply to LCD actions;
- sensor palette populated from live metrics;
- widget palette;
- media palette/library;
- 960×376 canvas scaled responsively without losing the logical coordinate system;
- inspector for selected layers;
- preview using the real Rust renderer.

Media must provide reusable image/animation assets, replacement, deletion protection and Orbit preset controls. Display must provide safe power control and software luminance, clearly labelled as software-only when native backlight control is unavailable.

Providers must be a real functional tab, not a placeholder. Each provider card shows enabled state, URL, provider-specific fields, credential state, TLS verification, connection test, latest collection status and last successful refresh where available.

The frontend may improve spacing, typography, responsive behavior, iconography and hierarchy over the old UI, but must not regress feature density or readability.

## Provider architecture
Provider configuration remains generic and private. Public source contains schemas and collectors only; actual URLs/secrets live in appdata.

Required providers:
- Proxmox: node status, memory, guests, disks and SMART data;
- local hardware/sysfs: CPU/GPU temperatures, GPU busy, VRAM and GTT/shared memory;
- Beszel: monitoring/system data where configured;
- Jellyfin and Silo: active playback/media state;
- Radarr and Sonarr: queue/incoming media;
- qBittorrent: transfer status/rate and incoming media;
- Immich: connection/status and useful photo/library metrics where available;
- Ollama: connection/version and useful model/runtime metrics where available.

Existing Rust media collectors remain and are extended, not replaced. Proxmox/local collection parity with the retired Python telemetry runtime is mandatory. Beszel/Immich/Ollama must be real collectors, not catalog-only entries.

## Data model and migration
Existing user pages and media must not be overwritten automatically.

Factory templates get explicit stable template IDs and a template version. New installations receive the six public templates. Existing installations may restore/upgrade a factory page deliberately through the existing restore flow; custom pages remain untouched.

The renderer must support the widget semantics needed by the restored layouts: text/value, gauge, horizontal/vertical bar, badge/state, image, animation and any decorative primitive required for faithful template rendering. Widget bindings resolve through the semantic metric catalog rather than provider-specific UI code.

Factory page restore must be deterministic and covered by golden/visual tests. The public templates must not contain private IDs, names, URLs or IPs.

## Media behavior
Media display priority remains:
1. currently playing;
2. incoming/downloading;
3. newly landed/completed;
4. idle/offline.

The Media page adapts its visible fields to the mode. It should show poster/art only when available and preserve layout balance when it is not.

## Branding
Public UI and public LCD templates use the AooScope name/logo only.

Private instance branding may exist only in instance appdata/config and must never be required by public source or tests.

## Testing and acceptance
The change is accepted only when all of the following are true:

- Rust fmt, Clippy, workspace tests, `cargo audit` and `cargo deny` pass;
- Svelte check, Vitest and production build pass;
- Playwright passes at 390, 768 and 1440 widths;
- visual screenshots of the public web UI are reviewed against the pre-cutover reference and are at least as polished;
- renderer tests cover all six templates at exactly 960×376;
- visual/golden checks prove the restored Home/Storage/Compute/Media compositions retain the intended hierarchy;
- provider unit tests cover normalization, auth failures, offline states and secret masking;
- provider integration tests use synthetic fixtures/mocks and never real credentials;
- public repo privacy scan finds no private installation identifiers;
- OCI first-run smoke and runtime executable scan remain green;
- production shadow smoke passes before production replacement;
- production APIs remain healthy, restart count remains zero and only safe known LCD operations are used.

## Deployment
Development occurs on a feature branch. Production remains untouched until CI and GHCR are green.

Before cutover, preserve the existing production appdata and active image. Deploy the new image first to a simulated shadow using a copy of production appdata. Only after shadow success should the real AooScope container be recreated.

After deployment, restore/upgrade the desired factory pages explicitly, keep private provider settings/secrets in appdata, apply the carousel, verify serial activity and safe power/luminance controls, then clean merged branches/staging artifacts.

## Non-goals
- no return to Python/Waitress/Node runtime;
- no host-networking or privileged container mode;
- no unknown AOOSTAR LCD protocol opcodes;
- no hardcoded private homelab topology in the public repository;
- no unrelated homelab migration work.
