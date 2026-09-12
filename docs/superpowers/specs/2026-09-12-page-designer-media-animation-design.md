# AooScope Page Designer, Media Library & Animation — Design

## Goal

Turn the current fixed `Splash / Home / Storage / Compute / Media` carousel into a user-editable 960×376 LCD composition system while preserving the existing optimized templates.

Users must be able to create, edit, duplicate, reorder, enable/disable and delete pages from the Admin UI. Existing system templates remain editable and can always be restored to their factory definition.

The editor must support drag-and-drop data widgets, uploaded images/logos/videos, reusable media assets, live preview, and safe deployment to the WTR MAX through the existing `aoostar-rs` rendering path.

## Scope

The first release covers:
- Page CRUD and carousel ordering.
- WYSIWYG editor at the exact 960×376 logical resolution.
- Data widgets bound to AooScope sensor keys.
- Static media assets and lightweight animation sources.
- Reusable templates and “Restore factory template”.
- Live preview and explicit Apply-to-LCD workflow.
- HTML/CSS/SVG animation authoring for supported animated assets.

Out of scope for v1: arbitrary JavaScript execution from uploaded user content, unrestricted remote URLs inside render definitions, or full-frame high-FPS video playback.
## Architecture

AooScope gains four bounded subsystems:

1. **Page Store** — persists page definitions in `/app/cfg/pages.json` using stable UUIDs, explicit z-order, carousel order and template provenance.
2. **Media Library** — stores uploaded assets under `/app/cfg/media/`, maintains metadata in `/app/cfg/media.json`, and never embeds binaries in page JSON.
3. **Designer API/UI** — Admin routes for pages, assets, sensor catalog, preview and apply. The browser edits a normalized page model instead of raw `aoostar-rs` config.
4. **Compiler/Renderer** — converts the normalized page model into generated backgrounds, dynamic visual assets and `monitor.json` compatible with `asterctl`.

`panels.py` stops being the source of truth for the active carousel. It becomes the provider of built-in factory templates plus compatibility helpers.

The existing supervisor remains the owner of `asterctl`. Applying a page revision writes files atomically, validates them, then asks the supervisor to reload. A failed compile keeps the previous known-good revision active.

## Persistence model

`pages.json` contains `schema_version`, `carousel`, and a dictionary of pages. Each page contains `id`, `name`, `enabled`, `duration`, `background`, `layers`, `template_id`, and `revision`.

Every layer contains common geometry (`x`, `y`, `width`, `height`, `rotation`, `opacity`, `z`) plus a typed payload. Coordinates are absolute LCD coordinates so preview and device rendering share one geometry model.

Factory templates use IDs such as `factory.home.v1`; user edits do not overwrite the factory source. “Restore” replaces the editable instance with a fresh copy of its factory template while preserving its page identity and carousel position.
## Layer types and data binding

Supported v1 layers:
- `text` — static text or formatted sensor value.
- `value` — large numeric/text value with unit and thresholds.
- `bar` — horizontal or vertical fill bar.
- `gauge` — semicircular or circular gauge.
- `ring` — compact radial progress indicator.
- `badge` — state/health chip with conditional color/icon.
- `image` — reusable media asset with contain/cover/crop.
- `sparkline` — short rolling history from a sensor series when available.
- `animation` — frame sequence produced from an approved animation source.

The Admin exposes a sensor catalog generated from current `state.json` plus registered provider schemas. Dragging a sensor onto the canvas creates a default `value` layer; users can then change its visual type without changing the binding.

Bindings use stable keys such as `aooscope_pve_cpu_pct`, not UI labels. Each widget declares formatting, min/max and optional threshold rules. Missing values render a configurable fallback (`--`, hidden, or badge state) rather than stale data.

Text always renders above its own gauge/bar graphics by construction. Compound widgets compile into distinct visual and text layers with the text z-order reserved above generated graphics.

## Editor UX

The Pages view shows ordered page thumbnails with drag handles, enable toggles, duration, duplicate, delete and restore actions. A persistent “Add page” control creates a blank page or starts from a template.

The editor uses a 960×376 canvas scaled responsively in the browser. Dragging, resizing and keyboard nudging modify exact logical coordinates. Optional grid snapping, alignment guides and safe margins reduce accidental overlap.

The left panel contains Sensors, Widgets and Media. The right inspector edits geometry, typography, colors, min/max, thresholds, formatting, asset fit/crop, opacity and z-order. The center canvas is the authoritative visual preview.
## Media Library

Uploads accept PNG, JPEG, WebP, SVG, GIF and common video formats. Files are validated by actual decoded type, dimensions and bounded size rather than trusting the filename or MIME header.

Static images are normalized for the LCD while preserving the original asset when useful. SVG is sanitized before preview/rasterization. Video and GIF uploads are treated as source media, not streamed directly to the LCD.

Each asset has an immutable ID and mutable display name. Replacing an asset creates a new revision while pages continue to reference the stable asset ID. Deleting an asset that is in use is blocked until references are removed or replaced.

Per-layer media properties support contain, cover, crop focal point, opacity and background transparency where supported.

## HTML logo and animation pipeline

The Admin may preview an animation authored as controlled HTML/CSS/SVG, including an animated AooScope/Cloud 9-style orbital logo. The WTR MAX does not execute HTML; animation is therefore compiled into LCD updates.

Animation sources are rendered in an isolated browser context to 960×376 frames. A frame optimizer compares successive frames, derives changed rectangles, merges nearby rectangles, and emits partial-update assets suitable for `aoostar-rs`.

The compiler enforces a configurable frame-rate/update budget. If an animation exceeds the serial bandwidth or changed-area budget, the UI reports the estimated cost and requires lowering FPS, reducing the animated region, or accepting a static fallback.

Full-frame video is allowed as an upload and preview source but is intentionally constrained on-device. The preferred target is localized animation (logo orbit, pulse, small waveform, status effect) rather than high-FPS full-screen playback.
## API surface

New Admin endpoints:
- `GET /api/pages` — list carousel order and page summaries.
- `POST /api/pages` — create blank/from-template page.
- `GET /api/pages/<id>` — full normalized page definition.
- `PUT /api/pages/<id>` — update page with revision check.
- `DELETE /api/pages/<id>` — delete non-required page.
- `POST /api/pages/<id>/duplicate` — duplicate page.
- `POST /api/pages/<id>/restore` — restore factory template.
- `PUT /api/carousel` — reorder/enable/duration changes atomically.
- `GET /api/sensors` — current sensor catalog, types and latest values.
- `GET/POST/PUT/DELETE /api/media...` — media library operations.
- `POST /api/preview` — compile one page to a browser preview without applying it.
- `POST /api/apply` — validate and publish a complete carousel revision.

Page updates use optimistic revision numbers so two browser tabs cannot silently overwrite each other. The API returns validation errors with layer/page IDs and human-readable messages.

## Apply and rollback

Editing is draft-first: browser changes are saved to page definitions but do not restart the LCD engine on every mouse move. Preview is cheap and local to the Admin.

“Apply to LCD” compiles all enabled pages into a staging directory, validates dimensions/assets/sensors, writes a staged `monitor.json`, then atomically promotes the revision. The previous compiled revision remains available for rollback.

The supervisor detects the promoted revision and reloads `asterctl`. If the engine fails its readiness check, AooScope automatically restores the previous revision and reports the error in the Admin.

Brightness remains an independent display transform applied after page composition, so existing brightness schedules work for both factory and custom pages.
## Security and limits

Uploaded content is treated as untrusted. Filenames are replaced by generated IDs, paths are resolved beneath the media root, decoded dimensions and file sizes are capped, and unsupported formats are rejected.

SVG is sanitized; arbitrary uploaded HTML/JavaScript is not executed in the main Admin origin. Animation authoring uses a constrained built-in template/runtime in a sandboxed renderer with no provider secrets, filesystem access or unrestricted network access.

Provider secrets remain in `/app/cfg/private/providers.json` or external secret files and are never embedded in `pages.json`, previews, exported templates or public API responses.

The compiler rejects widgets outside the 960×376 bounds unless explicitly clipped, malformed colors/fonts, duplicate layer IDs, invalid z-order, missing asset references and unknown sensor bindings.

Media quotas and per-upload limits are configurable. Generated preview/frame caches are bounded and disposable; source pages and media remain persistent.

## Migration

On first run without `pages.json`, AooScope creates editable instances of the current Splash, Home, Storage, Compute and Media templates. Their appearance and carousel order match the current production behavior.

Existing `settings.json`, provider secrets, custom Cloud 9 splash and brightness schedule are untouched. The current custom splash becomes a media-library asset referenced by the migrated Splash page.

The old hard-coded monitor builder remains as a compatibility/factory-template source during the migration release, but active rendering uses the new normalized page compiler after successful migration.

No migration deletes original user media. A timestamped page-definition backup is written before schema upgrades.
## Testing strategy

Unit tests cover page-schema validation, migration, asset lifecycle, sensor bindings, compiler geometry, z-order, brightness transforms, factory restore, animation diff rectangles and bandwidth budgeting.

API tests cover CRUD, revision conflicts, upload validation, media reference protection, preview, apply/rollback and secret non-disclosure.

Frontend tests cover page ordering, drag/drop coordinates, widget type conversion, layer ordering, media placement, inspector updates and restoration of factory templates. Served JavaScript continues to pass a syntax check in CI.

Golden-image tests compare representative 960×376 renders for factory pages and custom widgets. Device simulation validates generated `monitor.json` with `asterctl --simulate` before production deployment.

A hardware acceptance pass on Cloud 9 verifies page creation/edit/delete, upload reuse across multiple pages, drag/drop values, text foreground behavior, custom splash, brightness schedule, animation partial updates, carousel reload and automatic rollback without rebooting PVE or CT130.

## Success criteria

A non-technical user can build a new page entirely from the Admin UI, drag a live metric onto it, choose a visualization, add an uploaded image, preview it, place it in the carousel and apply it to the physical LCD without editing JSON or rebuilding Docker.

All current Cloud 9 pages remain visually available and restorable. Existing providers/settings survive migration. The public Docker image stays pull-only and generic, with no Cloud 9 or personal data committed to Git.

An animated orbital logo can be previewed as HTML/CSS/SVG in the Admin and reproduced on the WTR MAX using optimized partial updates within a measured safe update budget; unsupported high-bandwidth animations degrade explicitly rather than silently overloading the serial display.
