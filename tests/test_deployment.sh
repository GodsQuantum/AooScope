#!/usr/bin/env bash
set -u
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
fail=0
check() { local name=$1; shift; if eval "$*"; then printf 'PASS: %s\n' "$name"; else printf 'FAIL: %s\n' "$name"; fail=1; fi; }
contains() { grep -Fq -- "$1" "$2"; }
not_contains() { ! grep -Fq -- "$1" "$2"; }
check canonical_dockerfile_contains 'grep -Fq "FROM rust:1.98.1-trixie AS rust-base" Dockerfile'
check canonical_dockerfile_contains 'grep -Fq "FROM node:24.21.0-bookworm-slim AS frontend-check" Dockerfile'
check canonical_dockerfile_embeds_frontend 'grep -Fq "COPY --from=frontend-check /src/frontend/build ./frontend/build" Dockerfile'
check canonical_dockerfile_copies_one_binary 'grep -Fq "COPY --from=rust-builder /out/aooscope /usr/local/bin/aooscope" Dockerfile'
check runtime_packages_are_minimal 'grep -Eq "ca-certificates[[:space:]]+tini[[:space:]]+tzdata" Dockerfile'
check no_python_or_node_runtime '! awk "/^FROM debian/{runtime=1} runtime" Dockerfile | grep -Eiq "python|node"'
check no_waitress_or_asterctl_runtime '! grep -Fq waitress Dockerfile && ! grep -Fq asterctl Dockerfile'
check compose_is_pull_only 'not_contains "    build:" compose.yaml'
check exactly_one_compose 'test "$(find . -maxdepth 1 -type f \( -name "compose*.yaml" -o -name "compose*.yml" \) | wc -l)" -eq 1'
check compose_has_real_display_mode 'contains "AOOSCOPE_DISPLAY_MODE: real" compose.yaml'
check compose_has_generic_device 'contains "AOOSCOPE_DEVICE" compose.yaml && contains ":/dev/ttyACM0" compose.yaml'
check compose_has_config_volume 'contains "AOOSCOPE_CONFIG_DIR" compose.yaml && contains ":/app/cfg" compose.yaml'
check compose_has_rust_healthcheck 'contains "/usr/local/bin/aooscope" compose.yaml && contains "health" compose.yaml'
check compose_has_no_host_network 'not_contains network_mode compose.yaml && not_contains privileged compose.yaml'
check compose_has_no_legacy_environment 'not_contains AOOSCOPE_REFRESH_SECONDS compose.yaml && not_contains PVE_TOKEN_FILE compose.yaml'
check asterctl_dependency_is_pinned 'grep -Eq "asterctl-lcd.*git.*rev|git.*https://github.com/zehnm/aoostar-rs.*rev" crates/aooscope-display/Cargo.toml'
check cargo_lock_pins_asterctl 'grep -A5 -F "name = \"asterctl-lcd\"" Cargo.lock | grep -Eq "source = \"git\\+https://github.com/zehnm/aoostar-rs\\?rev=[0-9a-f]{40}"'
check health_api_is_declared 'contains "/api/health" crates/aooscope-server/src/app.rs'
check smoke_routes_are_declared 'contains "/api/status" crates/aooscope-server/src/app.rs && contains "/api/pages" crates/aooscope-server/src/app.rs && contains "/api/metrics" crates/aooscope-server/src/app.rs && contains "/api/media" crates/aooscope-server/src/app.rs'
check no_legacy_runtime_paths 'test ! -e aooscope && test ! -e webui.py && test ! -e web && test ! -e start.sh && test ! -e Dockerfile.python'
check no_python_tests 'test -z "$(find tests -type f \( -name "test_aooscope_*.py" -o -path "tests/parity/*" -o -name "designer_model_test.mjs" -o -name "test_repo_hygiene.py" \))"'
exit "$fail"
