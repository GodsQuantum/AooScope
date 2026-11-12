# Host deployment helpers

These files are generic host-side AooScope storage overlay deployment examples.

## Storage polling policy

AooScope keeps CPU/RAM/guest telemetry live, but storage inventory and SMART are intentionally rate-limited to one cycle per hour.

- App setting: `AOOSCOPE_STORAGE_POLL_SECONDS=3600`
- Host overlay timer: `aooscope-host-storage.timer` with `OnUnitActiveSec=1h`
- SMART failures keep the previous cached value instead of hammering Proxmox with retries.

## Files

- `systemd/aooscope-host-storage.timer`
- `systemd/aooscope-host-storage.service`
- `bin/aooscope-host-storage-snapshot`

The host service is low priority (`Nice=10`, idle I/O scheduling) and only refreshes filesystem-capacity metadata.