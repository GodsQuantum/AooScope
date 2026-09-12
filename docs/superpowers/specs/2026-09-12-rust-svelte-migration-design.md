# AooScope Rust + Svelte Migration Design

## Status

Approved direction: replace the Python runtime with a Rust backend and replace the vanilla-JS admin with Svelte, while preserving AooScope's public behavior and persisted data contracts.

This is a compatibility-first migration, not a clean-slate rewrite.

## Goals

- Ship one Rust application process in production.
- Serve a compiled Svelte admin from the Rust binary.
- Integrate `aoostar-rs` directly instead of spawning `asterctl` long-term.
- Preserve existing appdata and REST behavior during migration.
- Keep the current implementation as a reference oracle until parity is proven.
- Improve startup time, memory use, internal observability, and animation latency.
- Preserve safe draft/apply/rollback semantics for LCD page editing.

## Non-goals

- No Node.js runtime in production.
- No SSR requirement for the admin UI.
- No incompatible appdata migration unless explicitly versioned.
- No redesign of the LCD protocol.
- No arbitrary user JavaScript execution.
- No provider credential exposure to the frontend.

## September 2026 Technology Baseline

The migration targets stable releases current as of September 12, 2026, avoiding development/pre-release dependencies in production:

- Rust 1.98.1 toolchain, pinned with `rust-toolchain.toml`; 1.98.0 is not accepted because 1.98.1 fixes a vtable miscompilation.
- Tokio 1.53.1 async runtime, Axum 0.8.9 with Tower/Tower HTTP middleware, and Tower HTTP 0.7.1 where dependency compatibility permits.
- Reqwest 0.13.5 with Rustls TLS; Rustls 0.23.44 stable, not the 0.24 development line.
- Serde 1.0.229 and explicit schema-versioned JSON models; tracing 0.1.44 and thiserror 2.0.20 for structured diagnostics/errors.
- Svelte 5 runes with SvelteKit and `@sveltejs/adapter-static` 3.0.10; Node.js 24.21.0 LTS is the build-time JavaScript runtime and pnpm 12.3.4 the pinned package manager.
- Vite 8.1 using its Rolldown-based production pipeline; experimental bundled dev mode remains disabled unless profiling proves a benefit.
- Vitest 5 for frontend unit/component tests and Playwright 1.63 for browser tests.
- cargo-nextest 0.9.144 for Rust CI test execution; cargo-llvm-cov 0.9.x for dedicated coverage jobs.

Version floors are refreshed deliberately, not automatically. Lockfiles are committed and release/CI builds use `cargo --locked` and `pnpm --frozen-lockfile`.

## Target Architecture

A Cargo workspace provides the backend and shared domain model. Svelte is built as a static SPA and embedded into the Rust server.

```text
browser
  -> Svelte admin (static assets)
  -> REST / WebSocket
  -> Axum application
     -> settings/pages/media/revisions
     -> providers + telemetry
     -> page compiler + image renderer
     -> animation scheduler
     -> aoostar-rs
  -> AOOSTAR LCD
```

Production contains one main `aooscope` process. Background work runs as Tokio tasks under the same process with cancellation and health supervision.

## Workspace Boundaries

- `aooscope-types`: serde domain types shared across backend modules and exported to TypeScript.
- `aooscope-config`: appdata loading, validation, atomic writes, schema migration.
- `aooscope-providers`: Proxmox and application provider clients.
- `aooscope-telemetry`: sysfs and provider sampling, normalized sensor catalog.
- `aooscope-media`: upload validation, metadata, previews, SVG/video helpers.
- `aooscope-render`: 960x376 compositor, widgets, brightness and previews.
- `aooscope-display`: `aoostar-rs` integration, carousel, partial updates, animations.
- `aooscope-server`: Axum routes, WebSocket/SSE, static asset serving, health.
- `frontend`: Svelte admin compiled to static assets.

## Frontend Choice

Use SvelteKit with `adapter-static` as a prerendered static application. The root admin shell and any declared static routes are prerendered; no generic SPA fallback is generated initially. Axum serves the generated static output. There is no Node.js server in production.

Use Svelte 5 runes (`$state`, `$derived`, `$effect` only for true side effects). Page/editor selection that must survive reloads is represented in query/hash state rather than requiring server-side dynamic routes. The WYSIWYG uses Pointer Events plus CSS transforms during active drag/resize, schedules visual updates with `requestAnimationFrame`, and commits logical 960x376 geometry on pointer-up. Avoid a heavyweight canvas/editor dependency unless profiling proves it necessary.

The frontend owns presentation and interaction only:

- carousel/page management
- 960x376 WYSIWYG canvas
- drag/drop, resize, snapping and z-order
- inspector and undo/redo state
- Media Library
- provider/display settings
- previews and apply status

The backend remains authoritative for validation, compilation, persistence, secrets, provider access and LCD state.

The HTTP contract is generated from Rust using Utoipa OpenAPI. An `xtask api-schema` command emits the canonical OpenAPI document, and `openapi-typescript` generates frontend TypeScript definitions. The Svelte client uses a small project-local typed `fetch` wrapper; `openapi-fetch` is intentionally not adopted. `ts-rs` is not required unless a future non-HTTP shared model justifies it.

## Compatibility Contracts

The first Rust release must read the existing persisted files without user intervention:

- `settings.json`
- `pages.json`
- `media.json`
- `private/providers.json`
- `state.json`
- `compiled/current.json`
- `compiled/previous.json`
- compiled revision manifests/source documents

Existing media files remain valid. Secret files keep their current permissions and are never copied into public responses.

## HTTP/API Compatibility

The Rust server initially preserves the existing route surface and response semantics:

- `GET /`
- `GET/PUT /api/settings`
- `POST /api/providers/:name/test`
- `GET /api/status`
- `GET/POST /api/pages`
- `GET/PUT/DELETE /api/pages/:id`
- `POST /api/pages/:id/duplicate`
- `POST /api/pages/:id/restore`
- `PUT /api/carousel`
- `GET /api/sensors`
- `GET /api/media`
- `POST /api/media`
- `PUT /api/media/:id`
- `GET /api/media/:id/file`
- `DELETE /api/media/:id`
- `POST /api/preview`
- `POST /api/apply`
- `GET /api/health`

`GET /api/events` provides Server-Sent Events for one-way telemetry/display/status updates. REST remains the durable control plane. WebSocket is reserved for a future genuinely bidirectional low-latency feature and is not part of the initial migration.

## Data Integrity and Revisions

All persistent JSON writes are crash-consistent: write a temporary file in the destination directory, flush and fsync it, rename atomically, then fsync the parent directory. Schema versions remain explicit.

Page editing keeps the existing model:

1. Svelte edits a draft.
2. Draft saves do not affect the LCD.
3. Apply compiles a new immutable revision.
4. Validation and compilation must succeed before promotion.
5. The display switches to the promoted revision.
6. A failed display start marks the revision failed and restores the previous pointer.

Brightness changes re-render the currently applied revision and never implicitly promote a draft.

## Rendering and Media

The Rust renderer must preserve the current logical 960x376 canvas and widget behavior. Rendering uses Rust-native libraries (`image`/`imageproc` as appropriate, `resvg`/`tiny-skia` for SVG). FFmpeg remains an external helper for bounded video frame extraction rather than adding heavy unsafe bindings.

CPU-heavy decoding, rasterization and rendering never run on Tokio core worker threads. They run through `spawn_blocking` behind a bounded semaphore. Image decoders use explicit byte/pixel limits; potentially panicking decoder calls are isolated with unwind-safe error conversion. FFmpeg/helper subprocesses have input limits and hard execution timeouts.

Supported layer behavior remains compatible with the current page model:

- text/value
- horizontal or vertical bar
- gauge/ring
- badge
- sparkline
- image
- animation

Media uploads keep content-based validation, generated storage names, SHA-256 metadata, reference protection and bounded image dimensions.

## Display and Animation

The final runtime links the exact tested `aoostar-rs` Git commit directly as a Rust dependency, wrapped behind an internal `DisplayDriver` trait. The compatibility phase may temporarily provide both a CLI-backed adapter and a direct-library adapter for deterministic A/B comparison, but the target has no child `asterctl` process. Updating the pinned protocol implementation requires simulator and physical-LCD gates.

Animation uses small partial LCD updates. The animation scheduler owns a bounded update budget and never streams full-screen video at uncontrolled frame rates.

The browser may render richer CSS/SVG previews, but every LCD animation must compile to a deterministic display primitive supported by the backend.

## Providers and Telemetry

Tokio tasks collect sysfs and provider data on independent cadences. One failing provider cannot stall the telemetry loop. Snapshot state is distributed internally with `tokio::sync::watch` (and bounded broadcast only where event history is required), feeding both SSE and the display engine without filesystem polling. Secrets stay server-side and public API models expose only masked/set-state metadata.

Provider clients reuse long-lived `reqwest::Client` instances for connection pooling, with explicit connect/read/total timeouts, TLS verification by default, optional configured CA files, and bounded retry/backoff.

## Migration Strategy

Migration is vertical and contract-driven rather than a big-bang rewrite.

1. Freeze the Python/Svelte-predecessor behavior as fixtures and API contract tests.
2. Introduce Rust domain types and compatibility readers/writers.
3. Port settings, pages, revisions and media storage first.
4. Port renderer/compiler and compare generated configs/images against reference fixtures.
5. Port telemetry and providers with recorded responses plus live optional tests.
6. Integrate `aoostar-rs` directly and run display simulation/A-B tests.
7. Replace the admin with Svelte against the preserved REST contracts.
8. Build one Rust binary embedding the Svelte static output.
9. Run migration against a copied production appdata tree.
10. Deploy only after parity gates pass; retain the previous image tag for rollback.

Python remains in the repository only while it is needed as the reference oracle. It is removed from the production image once Rust reaches full parity.

## Error Handling and Observability

Use structured `tracing` events with module, provider and revision context. Health distinguishes HTTP readiness, telemetry freshness and display state.

No panic caused by malformed user configuration may terminate the service. Invalid persisted data returns actionable validation errors and leaves the last valid applied revision active.

Provider failures are isolated. Display failures trigger revision rollback where possible.

## Security

- No secrets in Svelte bundles, logs, public API payloads or repository fixtures.
- Upload paths never derive directly from client filenames.
- SVG sanitization remains mandatory before rasterization.
- API writes validate payload size and schema.
- Default deployment remains LAN/VPN scoped unless explicit authentication is configured.
- Container keeps `no-new-privileges` and minimal filesystem/device access.
- Rust dependencies are checked with `cargo audit`/`cargo deny`; release GitHub Actions are pinned to immutable commit SHAs.
- Stable crate releases are preferred; prerelease Rust dependencies require an explicit documented exception.

## Testing Strategy

Parity is proven at multiple levels:

- Rust unit tests for domain validation and persistence.
- Golden JSON tests for existing appdata formats.
- Golden image/config tests for page compilation.
- Recorded provider fixtures for deterministic client tests.
- API contract tests comparing Rust responses with the reference behavior.
- Vitest 5 component/model tests for editor operations.
- Playwright 1.63 browser smoke tests for page CRUD, media, providers, preview and apply, using `locator.drop()` for real DataTransfer/file drag-and-drop paths.
- cargo-nextest for normal Rust CI test execution; cargo-llvm-cov runs in a separate coverage job so instrumentation does not slow every PR gate.
- `aoostar-rs` simulation tests before physical display tests.
- Physical LCD validation only after all software gates are green.

A migration test copies a representative existing appdata tree, starts the Rust server against the copy, and verifies that no destructive rewrite occurs on startup.

## Build and Packaging

The development build has two toolchains: Cargo and Node/pnpm. Node is build-only. A workspace `xtask` crate provides stable entry points such as `cargo xtask api-schema`, `cargo xtask check` and `cargo xtask dist` so local and CI builds execute the same orchestration.

Frontend production builds use Vite 8.1/Rolldown. SvelteKit `adapter-static` prerenders the admin and generates Brotli + gzip assets. `rust-embed-for-web` embeds those precompressed assets with precomputed cache metadata; hashed Vite assets are served with immutable caching while the HTML entry document revalidates/no-caches. Zstd embedding stays disabled to avoid unnecessary native complexity for this small UI.

Docker uses BuildKit cache mounts for Cargo registry/git/target and pnpm stores as the baseline, plus external registry/GHA cache in CI. `sccache` 0.17 is used for local/CI Rust compilation where useful; `mold` 2.41 is preferred for Linux development/CI linking when available. `cargo-chef` is optional and is only added if measured Docker timings improve beyond BuildKit cache mounts.

Release builds use `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, symbol stripping, and panic unwinding. Public images remain generic x86_64 rather than `target-cpu=native`; machine-specific tuning may be an explicitly separate local artifact.

A multi-stage container builds Svelte, builds the Rust binary with the Svelte output embedded, then copies the binary plus only required runtime helpers/libraries into the final image.

Target production shape:

```text
/aooscope
/app/cfg/   # persistent volume
```

No Python interpreter, Flask, Waitress, npm, Node.js or separate animation process remains in the final runtime image.

## Release and Rollback

The Rust/Svelte image is published under a new prerelease tag first. Deployment preserves the existing appdata volume and keeps the previous known-good image tag available.

Rollback is a container image rollback, not a reverse data migration. Therefore the first Rust release may only write schemas that the previous stable release can still read, unless a versioned migration with a tested downgrade path is introduced.

## Acceptance Criteria

The migration is complete when all of the following are true:

- Existing appdata loads without manual conversion.
- Existing pages, media, brightness settings and provider configuration remain usable.
- Public secrets remain masked and private secrets remain server-side.
- Rust implements the current REST control surface with documented compatibility.
- Svelte provides page CRUD, carousel ordering, WYSIWYG editing, media, preview, display and provider controls.
- Apply creates and promotes an immutable compiled revision; a failed revision can roll back.
- The Rust renderer matches approved reference outputs within defined visual tolerances.
- Telemetry and provider behavior is failure-isolated and health-visible.
- LCD animations use bounded partial updates.
- Production starts as one main Rust process and requires no Python or Node runtime.
- Container health is green with zero unexpected restarts during validation.
- Physical LCD validation passes before the Rust/Svelte image becomes the stable tag.
- The previous stable container image can be restored against the same appdata.

## Repository Hygiene

All migration fixtures are generic and synthetic. No private hostnames, IP addresses, credentials, local paths or deployment-specific branding are committed to the public repository.

The current Python implementation is retained only as long as needed to establish parity and is removed from the production runtime before the migration is declared complete.
