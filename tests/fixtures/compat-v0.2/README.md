# Compatibility v0.2 goldens

`goldens.json` is a frozen compatibility snapshot captured from the v0.2
Python implementation before the Rust cutover. The legacy generator and
runtime are intentionally not part of this repository or CI.

The generator uses fixed fixture IDs and inputs. No secrets are emitted. The
only normalization is omission of generated page IDs from factory semantics;
route goldens use the fixture's stable IDs. Renderer image bytes are recorded
as an informational legacy hash, while parity tests compare stable dimensions,
panel fields, and semantic text—not raster identity across implementations.
Rust may add its `presets` field to `/api/media`; the legacy `assets` payload
remains exact. `crates/aooscope-server/tests/compat_goldens.rs` is the only
reader and keeps the snapshot executable without Python.
