#!/bin/bash
set -u

echo "=== AOOSTAR LCD Manager / Cloud 9 V2 ==="
echo "Démarrage des services..."

if [ ! -e /dev/ttyACM0 ]; then
    echo "⚠️ /dev/ttyACM0 non trouvé - l'écran ne sera pas contrôlé"
fi

python3 /app/cloud9_telemetry.py &
echo "✅ cloud9-telemetry démarré"

aster-sysinfo --refresh 5 \
    -o /app/cfg/sensors/sysinfo.txt \
    --temp-dir /app/cfg/sensors/ &
echo "✅ aster-sysinfo démarré"

sleep 3

# Helper historique conservé uniquement pour compatibilité native PVE.
if command -v qm >/dev/null 2>&1 && command -v pct >/dev/null 2>&1 && [ -d /sys/class/net/vmbr0 ]; then
    bash /app/proxmox-sensors.sh &
    echo "✅ proxmox-sensors démarré"
else
    echo "ℹ️ proxmox-sensors ignoré: Cloud 9 V2 utilise les sources CT130/API"
fi
if [ -e /dev/ttyACM0 ]; then
    asterctl \
        --config-dir /app/cfg \
        --config monitor.json \
        --font-dir /app/fonts \
        --sensor-path /app/cfg/sensors/ \
        --sensor-mapping /app/cfg/sensor-mapping.cfg &
    echo "✅ asterctl démarré"
fi

echo "✅ Démarrage du webui sur port 8765..."
python3 /app/webui.py
