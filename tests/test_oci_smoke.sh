#!/usr/bin/env bash
set -Eeuo pipefail
image=${1:?usage: $0 IMAGE}
port=${2:-18765}
config_dir=$(mktemp -d)
container=$(docker run -d --rm -p "127.0.0.1:${port}:8765" \
  -e AOOSCOPE_DISPLAY_MODE=simulated -e AOOSCOPE_CONFIG_DIR_IN_CONTAINER=/app/cfg \
  -v "${config_dir}:/app/cfg" "$image")
cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; rm -rf "$config_dir"; }
trap cleanup EXIT
for _ in $(seq 1 30); do
  if curl --fail --silent "http://127.0.0.1:${port}/api/health" >/dev/null; then break; fi
  sleep 1
done
for route in \
  /api/health \
  /api/status \
  /api/pages \
  /api/metrics \
  /api/media \
  /api/display/capabilities \
  /api/settings; do
  curl --fail --silent "http://127.0.0.1:${port}${route}" >/dev/null || {
    echo "FAIL: ${route}"; exit 1;
  }
done
echo "PASS: OCI API smoke ($image)"
