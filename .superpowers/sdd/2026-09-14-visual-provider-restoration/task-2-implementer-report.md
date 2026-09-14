# Task 2 implementer report

## Result

Implemented the rich 960×376 LCD renderer and the six approved public AooScope factory templates:

1. `factory.splash.v1` — enabled
2. `factory.home.v1` — enabled
3. `factory.storage.v1` — enabled
4. `factory.storage-m2.v1` — disabled by default
5. `factory.compute.v1` — enabled
6. `factory.media.v1` — enabled

First-run bootstrap installs the six pages in that order. Bootstrap remains write-once and its idempotence test proves existing settings, pages, media, state, secrets, and custom pages are not rewritten. Factory restore remains scoped to the selected factory page.

## Renderer work

- Replaced square gauge/ring fills with circular 230-degree gauges and full rings, including tracks, configurable thickness, progress, and rounded endpoints.
- Added reusable rounded rectangle geometry for bars, panels, borders, and badges.
- Added vertical/horizontal rounded bars with configurable ranges, tracks, and radii.
- Added badge text, border, alignment, vertical alignment, opacity, units, explicit text scale, and readable byte-rate formatting.
- Added nested object/array binding resolution while preserving flat underscore-key compatibility.
- Kept the legacy media `headline` and `title_short` bindings compatible with the current `mode` and `title` state fields.
- Added optional state-bound media assets; missing poster/art leaves the composed AooScope fallback intact without a warning.
- Preserved the exact RGB output dimensions at 960×376.

## Factory composition checklist

- Splash: AooScope-only brand, halo treatment, and compatibility with existing optional Orbit/media behavior.
- Home: header/accent hierarchy, CPU/RAM/temperature arc gauges, large values, guest count, and captions.
- Storage: six bordered SATA cards with disk name, bay, temperature, vertical thermal bar, and SMART health.
- Storage M.2: four NVMe cards with device, role, temperature, thermal bar, and health.
- Compute: GPU/CPU/shared-memory arc gauges plus GPU and CPU temperatures.
- Media: poster/art region with graceful fallback, status panel, mode, title, source, provider chain, progress, ETA, and formatted transfer rate.
- All factory source strings and previews use AooScope/generic synthetic content only.

## TDD evidence

Initial RED baseline on HEAD `86316c4`:

- `gauges_and_bars_use_rounded_geometry` failed because the gauge top pixel remained background.
- `nested_state_bindings_and_text_units_are_rendered` failed because nested array/object bindings were unresolved.
- `empty_root_bootstraps_all_simulated_api_reads` failed because `page-storage-m2` was absent.

After implementation, those three targeted tests pass. A new deterministic factory test was first run RED with zero placeholder hashes, reporting all six rendered hashes, then made GREEN with the approved output hashes. It verifies exact page order, stable template IDs, enabled defaults, exact binding sets, empty warnings, native dimensions, deterministic pixel hashes, and privacy markers.

## Preview evidence

Generated and visually inspected at native 960×376 under `Sources/Worklogs/task2-previews/` using generic synthetic state. The first inspection exposed suppressed short text plus an over-wide Media title/raw byte rate; those were corrected and all previews were regenerated and re-inspected.

| Preview | SHA-256 |
| --- | --- |
| `compute.png` | `47049c9b2460a347cb3ad1c5159a91a16748b73e50dd5a7fcad634f152384c28` |
| `home.png` | `d53550344cc5b8a36e6ff0a24d01ebfc6f5a093277c898ab10020cbc8eace35b` |
| `media.png` | `8ca587c05fe175cbb18db34079fabb03424ed5b9ebc4e232277a17d81cd07f13` |
| `splash.png` | `b14272b2cfd1c85cb4d63722bbe7a9743fcfd2aaa75ac77a7baceb11baf28cd2` |
| `storage-m2.png` | `0606e1f1a98bc2a383fdbf9f6a5de34a2b91bae8681f4fb189b504e3600247e0` |
| `storage.png` | `7058783f0e84a22fc81d1d33e8e838b5ef648c9f5c1d6afff740eb623b176011` |

## Validation evidence

- Targeted RED→GREEN renderer and bootstrap tests: pass.
- `cargo test -p aooscope-render --all-targets`: pass (13 tests).
- `cargo test -p aooscope-server --test bootstrap --test designer_api --test compat_goldens`: pass (10 tests).
- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace --all-targets`: pass (all workspace targets).
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo audit`: pass; one existing allowed `paste` unmaintained warning.
- `cargo deny check`: pass; configured duplicate-version warnings only.
- `git diff --check`: pass.
- Changed/public source privacy scan: no prohibited instance branding or private-network literal in shipped templates/source. Runtime-generated marker strings are used by the privacy regression test so the prohibited literals are not themselves shipped in tracked source.

Frontend Task 3, production, mounts, system configuration, sibling projects, containers, and remote machines were not touched.
