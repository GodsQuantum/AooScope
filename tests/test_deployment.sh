#!/usr/bin/env bash
set -u
fail=0
pass(){ printf 'PASS: %s\n' "$1"; }
fail_check(){ printf 'FAIL: %s\n' "$1"; fail=1; }
check(){ if eval "$2"; then pass "$1"; else fail_check "$1"; fi; }
PROD_DOCKERFILE=Dockerfile.python
RUST_DOCKERFILE=Dockerfile

check "production Docker build is multi-stage" "grep -Eq '^FROM rust:.*[Aa][Ss] builder' \"$PROD_DOCKERFILE\""
check "production aoostar-rs source is pinned" "grep -Eq '^ARG AOOSTAR_RS_REF=[0-9a-f]{40}$' \"$PROD_DOCKERFILE\""
check "production runtime copies generic package" "grep -Fq 'COPY aooscope/ /app/aooscope/' \"$PROD_DOCKERFILE\""
check "production runtime uses Waitress" "grep -Fq 'python3-waitress' \"$PROD_DOCKERFILE\" && grep -Fq 'waitress-serve' start.sh"
check "production runtime has no flask-cors dependency" "! grep -Fq 'flask-cors' \"$PROD_DOCKERFILE\""
check "production runtime uses generated defaults" "grep -Fq 'COPY defaults/ /app/defaults/' \"$PROD_DOCKERFILE\" && ! grep -Fq 'COPY cfg/' \"$PROD_DOCKERFILE\""
check "GHCR workflow still publishes Python production image" "grep -Fq 'file: Dockerfile.python' .github/workflows/container.yml"
check "Rust foundation pins Rust 1.98.1" "grep -Fq 'FROM rust:1.98.1-trixie AS rust-base' \"$RUST_DOCKERFILE\""
check "Rust foundation pins Node 24.21.0" "grep -Fq 'FROM node:24.21.0-bookworm-slim AS frontend-check' \"$RUST_DOCKERFILE\""
check "Rust runtime contains single AooScope binary" "grep -Fq 'COPY --from=rust-builder /out/aooscope /usr/local/bin/aooscope' \"$RUST_DOCKERFILE\""
check "public compose pulls GHCR image" "grep -Fq 'ghcr.io/godsquantum/aooscope' compose.yaml && ! grep -Eq '^[[:space:]]*build:' compose.yaml"
check "dev compose keeps local build" "grep -Eq '^[[:space:]]*build:' compose.dev.yaml"
check "compose does not use host networking" "! grep -Eq '^[[:space:]]*network_mode:[[:space:]]*host' compose.yaml"
check "compose device is configurable" "grep -Fq 'AOOSCOPE_DEVICE' compose.yaml"
check "compose config path is configurable" "grep -Fq 'AOOSCOPE_CONFIG_DIR' compose.yaml"
check "production runtime starts telemetry" "grep -Fq 'python3 -m aooscope.telemetry &' start.sh"
check "production runtime starts display supervisor" "grep -Fq 'python3 -m aooscope.supervisor &' start.sh"
check "production runtime starts animation engine" "grep -Fq 'python3 -m aooscope.animation_runtime' start.sh"
check "healthcheck covers telemetry/display/UI" "grep -Fq 'aooscope.telemetry' compose.yaml && grep -Fq 'aooscope.supervisor' compose.yaml && grep -Fq 'asterctl' compose.yaml"
check "admin JavaScript syntax is valid" "node --check web/admin.js >/dev/null"
check "no legacy host helper is shipped" "test ! -e proxmox-sensors.sh"
check "local data is gitignored" "grep -Fxq 'data/' .gitignore && grep -Fxq '.venv/' .gitignore"

exit "$fail"
