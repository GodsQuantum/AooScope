#!/bin/bash
PROXFILE="/app/cfg/sensors/proxmox.txt"
OUT="/app/cfg/sensors/values.txt"
while true; do
  VM_COUNT=$(qm list 2>/dev/null | grep -c running || echo 0)
  LXC_COUNT=$(pct list 2>/dev/null | grep -c running || echo 0)
  UPTIME=$(uptime -p | sed 's/up //')
  IP=$(ip addr show vmbr0 | grep "inet " | awk '{print $2}' | cut -d/ -f1)
  RX1=$(cat /sys/class/net/vmbr0/statistics/rx_bytes)
  TX1=$(cat /sys/class/net/vmbr0/statistics/tx_bytes)
  sleep 5
  RX2=$(cat /sys/class/net/vmbr0/statistics/rx_bytes)
  TX2=$(cat /sys/class/net/vmbr0/statistics/tx_bytes)
  DOWN=$(( (RX2 - RX1) / 1024 ))
  UP=$(( (TX2 - TX1) / 1024 ))
  cat > "$PROXFILE" << EOF
proxmox_ip: $IP
proxmox_vm_lxc: VMs:${VM_COUNT} LXC:${LXC_COUNT}
proxmox_uptime: $UPTIME
proxmox_net_down: Down:${DOWN}K/s
proxmox_net_up: Up:${UP}K/s
EOF
  cat "$PROXFILE" >> "$OUT"
done
