# Deployment

## Docker Compose

```bash
mkdir -p aooscope/data
cd aooscope
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/compose.yaml
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/.env.example
cp .env.example .env
docker compose up -d
```

The public Compose pulls `ghcr.io/godsquantum/aooscope:latest` and persists only `./data` by default.

## Device

The common WTR MAX device is `/dev/ttyACM0`. Verify it exists on the machine running Docker and adjust `AOOSCOPE_DEVICE` when necessary.

AooScope uses `aoostar-rs` at 960×376 and the protocol settings supported by that project. No kernel driver or AooScope host daemon is installed.

## Network

The Admin UI binds to `127.0.0.1:8765` by default. Set `AOOSCOPE_BIND_ADDRESS` to a trusted LAN address if needed. Do not expose it directly to the public internet because AooScope deliberately does not implement its own authentication layer.

## Branding

Upload or place a custom 960×376 image under the persistent `branding/` directory and set `AOOSCOPE_SPLASH_IMAGE` to the path relative to `/app/cfg`, for example:

```dotenv
AOOSCOPE_SPLASH_IMAGE=branding/my-server.jpg
```

Display brand, brightness and carousel interval can then be changed live from the Admin UI.

## Software brightness

AooScope currently implements pixel luminance rather than a physical backlight command. The Admin can apply a base percentage and time rules, including ranges crossing midnight.

Example: `22:00 → 08:00 = 70%` while the base level remains `100%` during the day.

## Updates

```bash
docker compose pull
docker compose up -d
```

Keep the `data/` directory when replacing containers; it contains settings, provider secrets and custom branding.
