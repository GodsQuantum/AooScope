#!/usr/bin/env bash
set -u
fail=0
check(){ if eval "$2"; then printf 'PASS: %s\n' "$1"; else printf 'FAIL: %s\n' "$1"; fail=1; fi; }
check "Docker build is multi-stage" "grep -Eq '^FROM rust:.*[Aa][Ss] builder' Dockerfile"
check "aoostar-rs source is pinned" "grep -Eq '^ARG AOOSTAR_RS_REF=[0-9a-f]{40}$' Dockerfile"
check "runtime copies generic package" "grep -Fq 'COPY aooscope/ /app/aooscope/' Dockerfile"
check "runtime uses production WSGI" "grep -Fq 'python3-waitress' Dockerfile && grep -Fq 'waitress-serve' start.sh"
check "runtime has no flask-cors dependency" "! grep -Fq 'flask-cors' Dockerfile"
check "runtime uses generated defaults, not legacy cfg" "grep -Fq 'COPY defaults/ /app/defaults/' Dockerfile && ! grep -Fq 'COPY cfg/' Dockerfile"
check "public compose pulls GHCR image" "grep -Fq 'ghcr.io/godsquantum/aooscope' compose.yaml && ! grep -Eq '^[[:space:]]*build:' compose.yaml"
check "dev compose keeps local build" "grep -Eq '^[[:space:]]*build:' compose.dev.yaml"
check "compose does not use host networking" "! grep -Eq '^[[:space:]]*network_mode:[[:space:]]*host' compose.yaml"
check "compose device is configurable" "grep -Fq 'AOOSCOPE_DEVICE' compose.yaml"
check "compose config path is configurable" "grep -Fq 'AOOSCOPE_CONFIG_DIR' compose.yaml"
check "runtime starts telemetry" "grep -Fq 'python3 -m aooscope.telemetry &' start.sh"
check "runtime starts display supervisor" "grep -Fq 'python3 -m aooscope.supervisor &' start.sh"
check "healthcheck covers telemetry/display/UI" "grep -Fq 'aooscope.telemetry' compose.yaml && grep -Fq 'aooscope.supervisor' compose.yaml && grep -Fq 'asterctl' compose.yaml"
check "no legacy host helper is shipped" "test ! -e proxmox-sensors.sh"
check "local data is gitignored" "grep -Fxq 'data/' .gitignore && grep -Fxq '.venv/' .gitignore"
exit "$fail"
