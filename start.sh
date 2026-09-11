#!/bin/bash
set -u

echo "=== AooScope ==="
echo "Starting telemetry, display engine and web UI..."

DEVICE="${AOOSCOPE_DEVICE:-/dev/ttyACM0}"
CONFIG_FILE="${AOOSCOPE_MONITOR_CONFIG:-monitor.json}"

python3 -m aooscope.telemetry &
echo "aooscope telemetry started"

if command -v aster-sysinfo >/dev/null 2>&1; then
    aster-sysinfo --refresh "${AOOSCOPE_SYSINFO_SECONDS:-5}" \
        -o /app/cfg/sensors/system.txt \
        --temp-dir /app/cfg/sensors/ &
fi

sleep 2

if [ -e "$DEVICE" ]; then
    asterctl \
        --device "$DEVICE" \
        --config-dir /app/cfg \
        --config "$CONFIG_FILE" \
        --font-dir /app/fonts \
        --sensor-path /app/cfg/sensors/ \
        --sensor-mapping /app/cfg/sensor-mapping.cfg &
    echo "display engine started"
else
    echo "display device not found: $DEVICE"
fi

exec python3 /app/webui.py
