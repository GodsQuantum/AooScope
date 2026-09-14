# AooScope Visual Studio Redesign — Design Spec

Date: 2026-09-14
Status: approved for implementation
Scope: public/generic AooScope product only

## Goal

Turn the existing admin-form designer into a canvas-first visual composition studio while preserving the current Rust/Axum + embedded SvelteKit + native AOOSTAR LCD architecture.

The redesign must make AooScope immediately understandable on a 960×376 display and in the browser: human metric labels, modern typography, compact storage capacity bars, rich media cards, visual templates, direct manipulation, and progressive disclosure of advanced settings.

## Non-negotiable architecture

- Production remains one application process: `tini -> aooscope`.
- Rust/Axum remains the backend and renderer host.
- SvelteKit remains prebuilt/static and embedded; no Node runtime in production.
- LCD remains native 960×376 through `asterctl-lcd`.
- No undocumented AOOSTAR opcodes; native brightness remains unsupported.
- Software luminance remains the only brightness control.
- Public Git must contain no private IPs, host names, credentials, private branding, or private media.
- Existing appdata/pages remain migration-compatible; no destructive reset during rollout.
## UX model

### Canvas-first editor

The 960×376 canvas is the dominant surface. The current permanent three-column palette/canvas/inspector layout is replaced by:

- a compact horizontal page strip with thumbnail/name/status;
- one large centered canvas;
- a bottom/left metric library opened on demand;
- a contextual quick toolbar for the selected layer;
- an advanced drawer only when requested.

Page operations such as enable, duration, duplicate, restore, delete and move are hidden behind a compact context menu or inline affordance instead of being repeated in every large page card.

The canvas never shows internal bindings such as `aooscope_media_display_headline`. It shows friendly metric labels and current/demo values.

### Direct manipulation

Every layer remains draggable and resizable. Selecting a layer exposes only high-frequency properties first: data source, style, color, representation and duplicate/delete. Exact x/y/width/height, z-index, raw binding, min/max and other plumbing live under Advanced.

Metric insertion is data-first: users choose `CPU temperature`, `RAM`, `Disk 1 used`, etc., then AooScope chooses the metric's recommended widget. The representation can then be switched among compatible styles.
## Visual template system

A template is a starting composition, never a locked mode. The picker shows miniature previews and human names.

Required templates:

1. **Semi rings** — large semicircular gauges with central value and short label.
2. **Vertical bars** — compact column metrics with value/label below.
3. **Horizontal bars** — dense rows optimized for CPU, RAM, queues and storage.
4. **Media card** — poster plus status/title/progress/ETA/source.
5. Existing Home and Compute compositions are modernized rather than removed.

Creating a page from a template instantiates ordinary editable layers. Switching a selected metric's representation changes the layer type/style without exposing an internal binding ID.

## Typography

The hand-built 5×7 bitmap font is removed from the normal text path. Rust uses an anti-aliased TrueType/OpenType rasterizer with bundled open-source Inter data supplied through a dependency, not a proprietary asset.

Initial implementation target:
- `fontdue 0.9.4` for deterministic glyph rasterization;
- `damascene-fonts-inter 0.1.0` for OFL-licensed Inter Variable bytes;
- Unicode text, lowercase, accents, punctuation, `°C`, alignment and ellipsis;
- no browser/headless renderer in production.

Legacy bitmap code may exist temporarily only behind tests during migration, then is deleted once the new renderer covers all text/value/badge paths.
## Storage inventory and pages

Storage must be driven by the actual telemetry inventory, not a fixed six-disk assumption.

The normalized storage model exposes, per physical device where available:
- stable path/id;
- human model/name;
- total capacity bytes;
- used/free bytes when the provider can determine them;
- usage percentage;
- temperature;
- SMART/health state;
- media type/role when known.

The metric catalog generates storage metrics from the full discovered device list instead of `(0..6)`.

Factory generation is deterministic:
- 0 devices: no auto-generated Storage page is enabled;
- 1–4 devices: one spacious page;
- 5–8 devices: one dense page or two pages when readability requires it;
- 9+ devices: pages are chunked deterministically, normally four to six devices per page.

NVMe devices may be grouped with other storage when this improves readability; a separate M.2 page is not mandatory.
### Storage card visual hierarchy

Capacity bars remain intentionally small and horizontal; they are not replaced by large gauges.

Each card/row prioritizes:
1. disk label/model;
2. `used / total` capacity and percentage;
3. a thin, high-contrast capacity bar;
4. temperature and health as secondary status.

Example semantic layout:

```text
MEDIAS 1     4.1 / 6 TB     34°   ✓
████████████████░░░░ 68%
```

The bar encodes capacity usage, never temperature. Temperature remains textual or a small secondary status signal.

## Rich media card

Media remains a signature page, not a generic set of metrics. It consumes the existing normalized `MediaDisplayEvent` and preserves provider fusion.

Visual priority:
1. active Jellyfin/Silo playback;
2. Radarr/Sonarr incoming content enriched by qBittorrent progress/speed/ETA;
3. qBittorrent-only activity;
4. recently landed media;
5. idle/offline fallback.
The 960×376 media composition uses a large portrait/poster region and a dominant status region containing:
- title, and season/episode context when available;
- provider chain in secondary text;
- progress bar;
- a large human ETA such as `READY IN 12 MIN` for incoming media;
- remaining playback time when playing;
- speed only when useful;
- a graceful placeholder when poster artwork is unavailable.

Poster handling must remain privacy-safe and robust: remote provider artwork is fetched/cached server-side into the existing media store rather than requiring the physical LCD renderer to perform arbitrary network fetches.

## Display settings

The Display tab uses progressive disclosure.

Always visible:
- power state/action;
- software luminance slider and percentage;
- carousel interval.

Collapsed by default:
- brightness schedule;
- timezone;
- brand string;
- advanced/display diagnostics.

No UI should imply native backlight support when `native_brightness=false`.
## Branding and animation extension point

Public AooScope supports a generic custom splash asset/animation extension point, but ships no private server branding.

The browser may preview a vector/HTML animation, while the physical LCD always receives native/pre-rendered frames through the existing Rust renderer/driver pipeline. A custom animation therefore has two representations with one visual language:
- browser: SVG/CSS/Web Animations or another lightweight vector runtime;
- LCD: imported/pre-rendered frame sequence or native Rust animation primitives.

The old visible `Orbit` concept is removed from primary UX. Existing private presets remain migration-safe until explicitly replaced.

## Responsive behavior

At 1440 px the page strip and tools surround a large canvas. At 768 px secondary controls become drawers/sheets. At 390 px the canvas remains fully visible at scaled width, with tools below it and no horizontal document overflow.

Nested scrolling is avoided. Search/result lists may scroll internally only when necessary; the page itself owns the primary vertical scroll.

## Accessibility and glanceability

- Minimum text contrast is checked on the real dark LCD palette.
- Color is never the only health/status signal.
- Interactive browser controls have labels and keyboard paths.
- LCD hierarchy favors one dominant value per module and avoids equal visual weight for every datum.
- Text truncates predictably with ellipsis rather than wrapping into unreadable multi-line blocks.
## Testing and rollout

Renderer tests cover Unicode/anti-aliasing behavior, bars/rings, storage rows, media card, and exact 960×376 output. Storage generation is tested with 0, 1, 4, 6, 8 and 9+ devices. Frontend unit tests cover metric insertion, friendly labels, representation switching, contextual/advanced controls and progressive disclosure.

Playwright is required at 390, 768 and 1440 px with screenshot review for Pages, Media and Display. No technical metric ID may appear in the normal canvas/editor path.

Before delivery all existing gates remain mandatory:
- `cargo fmt --all -- --check`;
- `cargo test --workspace --all-targets`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `pnpm check`, Vitest, build and Playwright;
- repo hygiene/privacy/deployment tests;
- `cargo audit`, `cargo deny check`, `git diff --check`;
- OCI build, smoke and runtime scan.

Deployment follows branch -> PR -> green CI -> merge -> GHCR digest -> shadow appdata -> backup -> pinned CT130 cutover -> API/provider/LCD verification. No production page reset is allowed merely to obtain new factory templates.

## Success criteria

A new user can add `CPU temperature`, a disk capacity row or a media widget without seeing internal bindings. The main editor feels canvas-led rather than form-led. The LCD no longer looks pixel-font based. All discovered disks fit readable auto-generated pages with visible capacity bars. Media activity shows poster/title/provider/progress and a human ETA or remaining time. Production remains one Rust process and the public repository stays generic/private-data-free.
