#!/usr/bin/env bash
set -u
fail=0
check() {
  if eval "$2"; then printf 'PASS: %s\n' "$1"; else printf 'FAIL: %s\n' "$1"; fail=1; fi
}
check "Docker build is multi-stage" "grep -Eq '^FROM rust:.*[Aa][Ss] builder' Dockerfile"
check "aoostar-rs source is pinned" "grep -Eq '^ARG AOOSTAR_RS_REF=[0-9a-f]{40}$' Dockerfile"
check "runtime copies generic package" "grep -Fq 'COPY aooscope/ /app/aooscope/' Dockerfile"
check "compose does not use host networking" "! grep -Eq '^[[:space:]]*network_mode:[[:space:]]*host' docker-compose.yml"
check "compose bind address is configurable" "grep -Fq 'AOOSCOPE_BIND_ADDRESS' docker-compose.yml"
check "compose device is configurable" "grep -Fq 'AOOSCOPE_DEVICE' docker-compose.yml"
check "compose config path is configurable" "grep -Fq 'AOOSCOPE_CONFIG_DIR' docker-compose.yml"
check "compose secrets path is configurable" "grep -Fq 'AOOSCOPE_SECRETS_DIR' docker-compose.yml"
check "runtime starts AooScope module" "grep -Fq 'python3 -m aooscope.telemetry &' start.sh"
check "asterctl reads sensor directory" "grep -Fq -- '--sensor-path /app/cfg/sensors/' start.sh"
check "no native PVE helper is started" "! grep -Fq 'proxmox-sensors' start.sh"
check "runtime contains upstream fonts" "grep -Fq 'COPY --from=builder /src/aoostar-rs/fonts/ /app/fonts/' Dockerfile"
check "local data is gitignored" "grep -Fxq 'data/' .gitignore && grep -Fxq 'secrets/' .gitignore"
exit "$fail"
