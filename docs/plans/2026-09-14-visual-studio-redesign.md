# AooScope Visual Studio Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved canvas-first AooScope redesign with modern LCD typography, dynamic storage pages/capacity bars, rich media presentation, simplified display controls, and generic custom splash animation support.

**Architecture:** Keep the existing single-process Rust/Axum runtime, embedded static SvelteKit frontend and native `asterctl-lcd` driver. Extend normalized telemetry and renderer capabilities behind stable APIs, then replace the current dense designer chrome with direct manipulation and progressive disclosure. Private deployment branding is applied only through ignored/private deployment assets after the generic product work is green.

**Tech Stack:** Rust 1.98.1, Axum, image, fontdue 0.9.4, damascene-fonts-inter 0.1.0, Svelte 5.57, SvelteKit 2.70, TypeScript 5.9, Vitest 5, Playwright 1.63, Docker/OCI.

**Spec:** `docs/design/2026-09-14-visual-studio-redesign.md`

## Global Constraints

- Production remains one process: `tini -> aooscope`; no Python, Node or headless browser runtime.
- LCD output is exactly 960×376 RGB through the existing Rust driver.
- No undocumented AOOSTAR opcode and no claim of native brightness support.
- Public Git contains no private IP, host name, credential, private deployment name, or private media asset.
- Existing appdata/pages are preserved; new factory behavior must not require destructive reset.
- Normal editor UI never displays raw `aooscope_*` bindings.
- Capacity is shown with small horizontal bars; temperature is not encoded as the capacity bar.
- Media presentation retains provider fusion and emphasizes poster/title/progress/human ETA or remaining time.

---
### Task 1: Modern anti-aliased typography

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/aooscope-render/Cargo.toml`
- Create: `crates/aooscope-render/src/typography.rs`
- Modify: `crates/aooscope-render/src/lib.rs`
- Modify: `crates/aooscope-render/src/compiler.rs`
- Test: `crates/aooscope-render/tests/compiler.rs`

**Interfaces:**
- Produces: `typography::draw_text(image, bounds, text, TextStyle)` and `typography::measure_text(text, TextStyle)`.
- `TextStyle` carries pixel size, color, horizontal/vertical alignment and optional max-lines/ellipsis behavior.
- `compiler.rs` routes text/value/badge text through this module; no 5×7 `glyph()` path remains in normal rendering.

- [ ] **Step 1: Write failing renderer tests** for lowercase/accented text (`"Température 42 °C"`), center/right alignment and ellipsis; assert non-background antialiased pixels include intermediate intensity values and exact canvas size remains 960×376.
- [ ] **Step 2: Run** `cargo test -p aooscope-render --test compiler typography -- --nocapture`; expect FAIL because the new typography path does not exist.
- [ ] **Step 3: Add exact dependencies** `fontdue = "0.9.4"` and `damascene-fonts-inter = "0.1.0"` as workspace dependencies and wire them into `aooscope-render`.
- [ ] **Step 4: Implement** an embedded Inter-backed rasterizer with alpha blending, clipping, alignment and ellipsis, then replace `draw_text()`/`glyph()` usage in `compiler.rs`.
- [ ] **Step 5: Run** targeted renderer tests, then `cargo fmt --all -- --check`, `cargo clippy -p aooscope-render --all-targets -- -D warnings`, and `cargo test -p aooscope-render --all-targets`.
- [ ] **Step 6: Commit** `feat: modernize lcd typography`.

---
### Task 2: Dynamic storage inventory and metrics

**Files:**
- Modify: `crates/aooscope-server/src/providers/telemetry.rs`
- Modify: `crates/aooscope-server/src/metrics.rs`
- Create: `crates/aooscope-server/src/storage.rs`
- Modify: `crates/aooscope-server/src/lib.rs`
- Test: `crates/aooscope-server/tests/metric_catalog.rs`
- Test: provider unit tests in `crates/aooscope-server/src/providers/telemetry.rs`

**Interfaces:**
- Produces: `storage::inventory(&StateDocument) -> Vec<StorageDevice>` where each item has index, path, label, kind, total_bytes, optional used/free/usage_pct, temperature and health.
- Proxmox normalization preserves `size` and `type` from `/nodes/{node}/disks/list`; optional provider fields `used`, `avail`, `usage_pct` are accepted when present.
- `metric_catalog()` emits name/size/used/free/usage/temperature/health metrics for every discovered device, with no fixed six-device cap.

- [ ] **Step 1: Add failing tests** using 0, 1, 8 and 10 disk fixtures; assert the metric count follows the actual array length and includes `aooscope_pve_disks_7_size_bytes` for the eighth disk.
- [ ] **Step 2: Add failing normalization test** asserting Proxmox disk `size` and `type` survive normalization and optional usage fields produce a calculated percentage.
- [ ] **Step 3: Run** `cargo test -p aooscope-server metric_catalog storage -- --nocapture`; expect failures from the fixed `(0..6)` catalog and missing normalized fields.
- [ ] **Step 4: Implement `StorageDevice` inventory** with safe optional calculations and update `normalize_proxmox()` plus `storage_metrics()` to iterate the inventory.
- [ ] **Step 5: Run** targeted tests plus server Clippy/tests; preserve the legacy plural binding `aooscope_pve_disks_{n}_name`.
- [ ] **Step 6: Commit** `feat: make storage inventory dynamic`.

---
### Task 3: Storage templates and compact capacity bars

**Files:**
- Create: `crates/aooscope-server/src/templates.rs`
- Modify: `crates/aooscope-server/src/routes/designer.rs`
- Modify: `crates/aooscope-server/src/bootstrap.rs`
- Modify: `crates/aooscope-render/src/compiler.rs`
- Test: `crates/aooscope-server/tests/bootstrap.rs`
- Test: `crates/aooscope-server/tests/designer_api.rs`
- Test: `crates/aooscope-render/tests/compiler.rs`

**Interfaces:**
- Produces: `templates::storage_pages(&[StorageDevice]) -> Vec<Page>` and reusable `storage_card_layers(device, bounds)`.
- Page chunking is deterministic and capacity bars bind to `aooscope_pve_disks_{n}_usage_pct`; temperature remains a separate value.
- New installs generate enough storage pages for discovered devices when state is available; existing customized pages are never silently overwritten.

- [ ] **Step 1: Add failing template tests** for 0/1/4/6/8/10 devices, checking page count, unique IDs, exact 960×376 geometry and no out-of-bounds layers.
- [ ] **Step 2: Add failing renderer test** for a thin horizontal storage bar at 68% and assert its fill length corresponds to usage, not temperature.
- [ ] **Step 3: Run** the targeted server/renderer tests and record the expected failures.
- [ ] **Step 4: Extract generic factory builders** from `routes/designer.rs` into `templates.rs`; add `factory.vertical-bars.v1`, `factory.semi-rings.v1`, `factory.horizontal-bars.v1`, and dynamic storage card helpers.
- [ ] **Step 5: Add safe storage-page generation** that only auto-creates missing factory-managed pages; keep restore semantics and legacy template IDs readable.
- [ ] **Step 6: Run** bootstrap/designer/renderer tests plus `validate_document` coverage and commit `feat: add adaptive storage templates`.

---
### Task 4: Canvas-first Visual Studio frontend

**Files:**
- Modify: `frontend/src/routes/+page.svelte`
- Modify: `frontend/src/lib/designer/Canvas.svelte`
- Replace: `frontend/src/lib/pages/CarouselSidebar.svelte` with compact `PageStrip.svelte`
- Create: `frontend/src/lib/designer/MetricLibrary.svelte`
- Create: `frontend/src/lib/designer/ContextToolbar.svelte`
- Create: `frontend/src/lib/designer/AdvancedInspector.svelte`
- Create: `frontend/src/lib/designer/TemplatePicker.svelte`
- Modify: `frontend/src/lib/designer/model.ts`
- Test: component tests under `frontend/src/lib/designer/*.test.ts`
- Test: `frontend/src/routes/page.test.ts`
- Test: `frontend/tests/designer.spec.ts`

**Interfaces:**
- `MetricLibrary` emits `{ metric, widgetType }` using friendly `Metric` descriptors.
- `ContextToolbar` edits representation/color/style for the selected layer without exposing raw bindings.
- `AdvancedInspector` is collapsed/closed by default and owns exact geometry/raw binding/min/max/z-index.
- `TemplatePicker` creates pages by server `template_id`; it displays miniature previews and human names.

- [ ] **Step 1: Write failing Svelte tests** asserting raw `aooscope_*` IDs are absent from normal editor text, metric insertion uses the friendly label/value, and no empty permanent Inspector column exists.
- [ ] **Step 2: Extend Playwright expectations** at 390/768/1440: canvas visible, compact page strip usable, `+ Metric`, Templates, Preview and Apply accessible, no horizontal overflow.
- [ ] **Step 3: Run** `pnpm test --run` and the targeted Playwright file; capture the failures before implementation.
- [ ] **Step 4: Implement the new composition shell** with large canvas, compact page strip, on-demand metric/template surfaces, contextual toolbar and advanced drawer while preserving current save/apply/preview API behavior.
- [ ] **Step 5: Make Canvas labels semantic** by resolving each binding against the metric catalog; only Advanced may show the technical ID.
- [ ] **Step 6: Run** `pnpm check`, `pnpm test --run`, `pnpm build`, `pnpm test:e2e -- designer.spec.ts`; review 390/768/1440 screenshots and commit `feat: redesign visual studio editor`.

---
### Task 5: Rich media card and poster pipeline

**Files:**
- Modify: `crates/aooscope-types/src/media_state.rs`
- Modify: `crates/aooscope-server/src/providers/http.rs`
- Modify: `crates/aooscope-server/src/providers/media.rs`
- Modify: `crates/aooscope-server/src/state.rs`
- Modify: `crates/aooscope-render/src/media.rs`
- Modify: `crates/aooscope-server/src/routes/designer.rs` or extracted template module
- Modify: `frontend/src/lib/media/MediaCard.svelte`
- Modify: `frontend/src/lib/media/MediaDashboard.svelte`
- Test: `crates/aooscope-server/tests/media_collectors.rs`
- Test: renderer/compiler and frontend media component tests

**Interfaces:**
- Extend `MediaDisplayEvent` with optional `remaining_minutes` and `poster_asset_id` while preserving existing JSON compatibility.
- `HttpClient` gains a bounded binary GET helper; live poster caching writes/replaces one deterministic media asset rather than leaking unbounded files.
- Jellyfin/Silo playing events calculate remaining time; Radarr/Sonarr/qBittorrent keep merged human ETA/progress/speed.

- [ ] **Step 1: Add failing collector tests** for Jellyfin/Silo remaining minutes and merged Radarr/Sonarr + qBittorrent ETA/provider chains.
- [ ] **Step 2: Add failing poster-cache test** using a bounded fixture image and assert the enriched event receives a resolvable `poster_asset_id` without exposing credentials in URLs/errors.
- [ ] **Step 3: Add failing frontend/render tests** asserting incoming media renders a poster region, dominant `READY IN N MIN`, title/provider chain and progress; playing renders remaining time instead of download language.
- [ ] **Step 4: Implement event fields, binary fetch, deterministic poster upsert/cache and media template/frontend hierarchy.** Poster failure must degrade to placeholder without failing the media poll.
- [ ] **Step 5: Run** media collector, renderer, frontend unit and E2E tests; verify no secret-bearing URL is persisted.
- [ ] **Step 6: Commit** `feat: enrich media presentation`.

---
### Task 6: Progressive Display UI and generic splash animation

**Files:**
- Modify: `frontend/src/lib/display/DisplayControls.svelte`
- Modify: `frontend/src/lib/media/MediaLibrary.svelte`
- Modify: `frontend/src/routes/+page.svelte`
- Modify: `crates/aooscope-render/src/compiler.rs`
- Modify: `crates/aooscope-render/src/media.rs`
- Modify: `crates/aooscope-server/src/routes/designer.rs` or extracted template module
- Test: display/media Svelte component tests
- Test: `crates/aooscope-render/tests/compiler.rs`

**Interfaces:**
- Display always shows power, software luminance and carousel interval; schedule/timezone/brand/diagnostics live in collapsed sections.
- Public UX replaces visible `Orbit` wording with generic `Use as splash` / `Splash animation` behavior while keeping old preset/API compatibility for migration.
- `animation` layers render actual time-varying raster frames for supported animated assets; the old 8×8 orbiting square effect is removed.

- [ ] **Step 1: Write failing UI tests** proving advanced display controls are collapsed by default and `native_brightness=false` is described accurately.
- [ ] **Step 2: Write failing renderer test** with a two-frame animated fixture and compile at two phases; assert output differs while no synthetic orbit square is drawn.
- [ ] **Step 3: Run** targeted frontend/render tests and confirm failures.
- [ ] **Step 4: Refactor DisplayControls** to progressive disclosure and remove primary Orbit copy/actions from the Media UI without deleting compatibility endpoints.
- [ ] **Step 5: Implement frame-aware animation rendering** from the existing media store with bounded decoding and deterministic phase selection.
- [ ] **Step 6: Run** frontend/render suites and commit `feat: simplify display and splash animation`.

---
### Task 7: Private deployment branding handoff

**Files:**
- Public code: no private branding files
- Private implementation plan: ignored deployment workspace under `Sources/Plans/`

**Interfaces:**
- Public AooScope exposes only the generic splash animation mechanism delivered by Task 6.
- Deployment-specific artwork, names, HTML/CSS previews and rendered LCD frames remain outside Git.

- [ ] **Step 1: Execute the ignored private branding plan** only after the generic animation renderer is green.
- [ ] **Step 2: Verify** the generated animated raster is 960×376 and loads through the same media/splash path as any user asset.
- [ ] **Step 3: Run the public privacy scan** and assert no deployment-specific name, path or asset is tracked.

---
### Task 8: Full visual gates, OCI shadow and production cutover

**Files:**
- Modify tests/goldens/screenshots only as required by approved behavior
- Modify: `README.md` / `README.fr.md` only for public user-facing feature changes
- Deployment: CT130 compose/appdata outside Git

**Interfaces:**
- Release artifact remains the single Rust binary image with embedded frontend.
- Production image is pinned by commit tag plus exact digest.
- Existing provider secrets and customized pages are preserved; storage regeneration/private splash are applied explicitly after backup.

- [ ] **Step 1: Run complete local gates:** `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, frontend `pnpm check`, `pnpm test --run`, `pnpm build`, full Playwright 390/768/1440, repo hygiene/privacy/deployment tests, `cargo audit`, `cargo deny check`, and `git diff --check`.
- [ ] **Step 2: Build OCI image** and run `tests/test_oci_smoke.sh` plus runtime executable scan; verify runtime process set remains `tini -> aooscope`.
- [ ] **Step 3: Perform whole-branch review** against the approved design spec; fix Important/Critical findings and rerun affected gates.
- [ ] **Step 4: Push feature branch and open PR only after local gates pass; wait for CI/container workflows to succeed before merge.**
- [ ] **Step 5: After merge/GHCR publication, retrieve exact digest, create a fresh CT130 backup, and shadow-run the digest against copied appdata with no LCD device access.**
- [ ] **Step 6: Cut over CT130** to `sha-<commit>@sha256:<digest>`, verify health/status/capabilities/providers, then verify real LCD pages and animation without reboot or undocumented opcode.
- [ ] **Step 7: Apply the private deployment splash asset and storage-page regeneration explicitly after the generic runtime is healthy; verify media card/poster/ETA and disk capacity bars on the physical display.**
- [ ] **Step 8: Clean merged local branches, stale scratch/build artifacts and obsolete shadow images while retaining rollback backups; confirm `main == origin/main` and clean working tree.

---

## Plan completion definition

The plan is complete only when Tasks 1–8 are individually reviewed/committed, every mandatory gate is green, the public repo is privacy-clean, CT130 runs the pinned merged digest, the AOOSTAR LCD shows the new typography/storage/media visuals, and the private deployment animation is active without entering public Git.
