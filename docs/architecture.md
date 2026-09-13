# Architecture

AooScope is a single Rust process in one minimal container. Axum serves the
embedded Svelte application and JSON API; Tokio runs provider polling,
rendering, revision management and the display scheduler.

```text
Linux sysfs + provider APIs
            │
            ▼
   Rust state and collectors ──► metrics/events/API
            │                              │
            ▼                              ▼
  renderer + revision store        embedded Svelte UI
            │
            ▼
     pinned asterctl-lcd driver ──► AOOSTAR LCD (960×376)
```

Only `/app/cfg` is persistent. It contains settings, private provider
credentials, pages, media, revisions and cached provider data. Secrets are
kept separate from public settings responses.

The final image contains `ca-certificates`, `tini`, `tzdata` and one
`/usr/local/bin/aooscope` executable. The frontend is built with pinned
Node/pnpm and embedded into that executable at build time; Node is not in the
runtime image. The display driver is linked directly through the pinned
`asterctl-lcd` git dependency and does not launch an `asterctl` child process.

The public Compose file grants only the configured LCD device and `/app/cfg`.
It does not use host networking, privileged mode, the Docker socket or a host
filesystem mount beyond the config directory.

## Compatibility

The frozen `tests/fixtures/compat-v0.2/goldens.json` snapshot documents the
legacy API and rendering contract. Rust integration tests read it directly;
the historical generator is not required at runtime or in CI.
