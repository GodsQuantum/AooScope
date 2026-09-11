#!/usr/bin/env bash
set -u
fail=0
check() {
  if eval "$2"; then printf 'PASS: %s\n' "$1"; else printf 'FAIL: %s\n' "$1"; fail=1; fi
}
check "Docker build is multi-stage" "grep -Eq '^FROM rust:.*[Aa][Ss] builder' Dockerfile"
check "aoostar-rs source is pinned" "grep -Eq '^ARG AOOSTAR_RS_REF=[0-9a-f]{40}$' Dockerfile"
check "Compose does not use host networking" "! grep -Eq '^[[:space:]]*network_mode:[[:space:]]*host' docker-compose.yml"
check "Web UI binds only the CT LAN IP" "grep -Fq '192.168.1.212:8765:8765' docker-compose.yml"
check "Persistent config uses canonical appdata" "grep -Fq '/srv/lxc/administration/appdata/aoostar-lcd/cfg:/app/cfg' docker-compose.yml"
check "Sensor helper uses container paths" "grep -Fq '/app/cfg/sensors/values.txt' proxmox-sensors.sh && ! grep -Fq '/root/aoostar-rs' proxmox-sensors.sh"
check "Host-only Proxmox helper is gated" "grep -Fq 'command -v qm' start.sh && grep -Fq 'command -v pct' start.sh && grep -Fq '/sys/class/net/vmbr0' start.sh"
check "Runtime contains upstream fonts" "grep -Fq 'COPY --from=builder /src/aoostar-rs/fonts/ /app/fonts/' Dockerfile"
check "Telemetry module copied into runtime" "grep -Fq 'COPY cloud9_telemetry.py /app/' Dockerfile"
check "Telemetry starts before asterctl" "grep -Fq 'python3 /app/cloud9_telemetry.py &' start.sh"
check "asterctl reads sensor directory" "grep -Fq -- '--sensor-path /app/cfg/sensors/' start.sh"
exit "$fail"
