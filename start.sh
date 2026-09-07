#!/bin/bash
# Script de démarrage de tous les services AOOSTAR LCD

echo "=== AOOSTAR LCD Manager ==="
echo "Démarrage des services..."

# Vérifier que le device USB est présent
if [ ! -e /dev/ttyACM0 ]; then
    echo "⚠️  /dev/ttyACM0 non trouvé - l'écran ne sera pas contrôlé"
fi

# Démarrer aster-sysinfo en arrière-plan
aster-sysinfo --refresh 5 \
    -o /app/cfg/sensors/values.txt \
    --temp-dir /app/cfg/sensors/ &
echo "✅ aster-sysinfo démarré"

# Attendre que le fichier de valeurs soit créé
sleep 3

# Démarrer proxmox-sensors en arrière-plan
bash /app/proxmox-sensors.sh &
echo "✅ proxmox-sensors démarré"

# Démarrer asterctl en arrière-plan
if [ -e /dev/ttyACM0 ]; then
    asterctl \
        --config-dir /app/cfg \
        --config monitor.json \
        --sensor-path /app/cfg/sensors/values.txt \
        --sensor-mapping /app/cfg/sensor-mapping.cfg &
    echo "✅ asterctl démarré"
fi

# Démarrer le webui en premier plan (pour garder le container actif)
echo "✅ Démarrage du webui sur port 8765..."
python3 /app/webui.py
