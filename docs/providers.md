# Providers

Providers are optional. AooScope works with local Linux/sysfs information even when no external service is configured.

## Runtime providers

| Provider | Current use | Recommended credential |
| --- | --- | --- |
| Proxmox VE | host CPU/RAM, guests, storage, ZFS, SMART | dedicated read-only API token / `PVEAuditor` |
| Jellyfin | now playing, latest media, poster | API key |
| Silo | native playback sessions, poster, codec/quality | scoped API key/token |
| Radarr | movie download/import queue and poster | API key |
| qBittorrent | download progress, speed and ETA | LAN bypass or WebUI account |

## Configurable / connection-test providers

Beszel, Sonarr, Immich and Ollama can be stored and tested from the Admin UI. Their normalized data surfaces are intentionally kept separate so future pages do not couple the display renderer to raw service payloads.

## URL format

The Admin accepts either full URLs or bare `IP:port` values. A value such as `server.lan:8096` is normalized to `http://server.lan:8096`.

TLS verification is enabled by default. Disable it only for trusted private endpoints using self-signed certificates.

## Secrets

Secrets are stored in `/app/cfg/private/providers.json` with mode `0600`. `GET /api/settings` only exposes `secret_set: true/false` and never returns secret values.

Changing a provider in the Admin is hot-reloaded; restarting the container is not required.

### Proxmox secret files

For existing read-only credentials, AooScope can use mounted files instead of copying the token into the Admin database. Put the files inside the config volume and set, for example:

```env
PVE_TOKEN_FILE=/app/cfg/private/pve-token.json
PVE_CA_FILE=/app/cfg/private/pve-root-ca.pem
```

The Admin **Test connection** and the telemetry collector both honor the custom CA when TLS verification is enabled.
