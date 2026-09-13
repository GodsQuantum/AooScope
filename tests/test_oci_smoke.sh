#!/usr/bin/env bash
set -u
image=${1:?usage: $0 IMAGE}
port=${2:-18765}
config_dir=$(mktemp -d)
container=$(docker run -d --rm -p "127.0.0.1:${port}:8765" \
  -e AOOSCOPE_DISPLAY_MODE=simulated -e AOOSCOPE_CONFIG_DIR_IN_CONTAINER=/app/cfg \
  -v "${config_dir}:/app/cfg" "$image")
cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; rmdir "$config_dir" 2>/dev/null || true; }
trap cleanup EXIT
for _ in $(seq 1 30); do
  if curl --fail --silent "http://127.0.0.1:${port}/api/health" >/dev/null; then break; fi
  sleep 1
done
for route in health status pages metrics media; do
  curl --fail --silent "http://127.0.0.1:${port}/api/${route}" >/dev/null || {
    echo "FAIL: /api/${route}"; exit 1;
  }
done
echo "PASS: OCI API smoke ($image)"
