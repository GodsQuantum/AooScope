#!/bin/bash
set -euo pipefail
CFG="${AOOSCOPE_CONFIG_DIR_IN_CONTAINER:-/app/cfg}"
mkdir -p "$CFG/sensors" "$CFG/private" "$CFG/cache" "$CFG/branding"
if [ ! -e "$CFG/sensor-mapping.cfg" ]; then
  cp /app/defaults/sensor-mapping.cfg "$CFG/sensor-mapping.cfg"
fi

echo "=== AooScope ==="
python3 -m aooscope.telemetry &
echo "telemetry started"
if command -v aster-sysinfo >/dev/null 2>&1; then
  aster-sysinfo --refresh "${AOOSCOPE_SYSINFO_SECONDS:-5}" \
    -o "$CFG/sensors/system.txt" --temp-dir "$CFG/sensors/" &
fi
sleep 2
python3 -m aooscope.supervisor &
echo "display supervisor started"
exec waitress-serve --listen=0.0.0.0:8765 webui:app
