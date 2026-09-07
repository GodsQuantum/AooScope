# AOOSTAR WTR MAX — Proxmox LCD Screen Manager

> Full control of the AOOSTAR WTR MAX built-in LCD screen under Proxmox VE, with a visual web configuration editor.

> 📸 **Screenshots coming soon** — Feel free to submit yours via Pull Request!

## 🖥️ Overview

This project enables real-time system information display on the AOOSTAR WTR MAX front LCD panel, running directly on Proxmox VE. It includes a complete web-based visual editor to customize the display without any coding.

**3 rotating panels:**
- **Panel 1** — CPU temp & usage, RAM, GPU, network speed, IP, time
- **Panel 2** — SSD/HDD temperatures and usage bars
- **Panel 3** — Proxmox dashboard: local IP, VMs/LXC count, uptime, network traffic

> 📸 **Panel screenshots coming soon**

---

## ✨ Features

- 📊 Real-time system metrics display
- 🎨 Visual web editor (AOOSTAR Screen Editor v2)
- 🖱️ Drag & drop elements on the preview
- 📐 Configurable snap-to-grid
- 🎨 Per-element color picker
- 🖼️ Background image upload (auto-resize to 960×376)
- ➕ Add/remove panels and elements
- 📋 Duplicate elements
- ↩️ Undo/Redo (Ctrl+Z, 30 levels)
- 💾 Export/Import JSON configuration
- 👁️ Live Preview with real sensor values
- ⏱️ Configurable transition duration between panels
- 🚀 Auto-start via systemd or Docker
- 📡 Proxmox-specific metrics (VMs, LXC, uptime, network)

---

## 🛠️ Requirements

- AOOSTAR WTR MAX (or compatible)
- Proxmox VE 8.x or 9.x
- Internet connection for initial setup
- Docker (optional, for Docker installation)

---

## 📦 Installation

### Method 1 — Native (systemd)

#### 1. Fix apt repositories (if you get 401 errors)

```bash
echo "# disabled" > /etc/apt/sources.list.d/pve-enterprise.list
echo "# disabled" > /etc/apt/sources.list.d/ceph.list
echo "# disabled" > /etc/apt/sources.list.d/ceph.sources
echo "# disabled" > /etc/apt/sources.list.d/pve-enterprise.sources
echo "deb http://download.proxmox.com/debian/pve trixie pve-no-subscription" > /etc/apt/sources.list.d/pve-no-sub.list
apt update && apt full-upgrade -y
```

#### 2. Install dependencies

```bash
apt install -y curl build-essential pkg-config libudev-dev git \
    python3-pil python3-flask python3-flask-cors
```

#### 3. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Choose 1 (standard installation)
source $HOME/.cargo/env
```

#### 4. Clone and compile asterctl

> ⚠️ **Important**: The pre-built v0.2.0 binary does NOT support `--sensor-mapping`.
> You MUST compile from source.

```bash
cd /root
git clone https://github.com/zehnm/aoostar-rs.git
cd /root/aoostar-rs
cargo build --release
# Takes 5-10 minutes
cp target/release/asterctl /usr/local/bin/
cp target/release/aster-sysinfo /usr/local/bin/
```

#### 5. Clone this repository

```bash
git clone https://github.com/YOUR_USERNAME/aoostar-proxmox-lcd.git
cd aoostar-proxmox-lcd
```

#### 6. Set up configuration files

```bash
cp cfg/sensor-mapping.cfg /root/aoostar-rs/cfg/sensor-mapping.cfg
cp cfg/sensor-mapping-filter.cfg /root/aoostar-rs/cfg/sensor-mapping-filter.cfg
cp cfg/monitor.json /root/aoostar-rs/cfg/monitor.json
cp webui.py /root/aoostar-rs/webui.py
cp proxmox-sensors.sh /root/aoostar-rs/proxmox-sensors.sh
chmod +x /root/aoostar-rs/proxmox-sensors.sh
```

#### 7. Edit sensor mapping for your NVMe

Find your NVMe name:
```bash
cat /root/aoostar-rs/cfg/sensors/values.txt | grep temperature_nvme_Composite
```

Edit the mapping:
```bash
nano /root/aoostar-rs/cfg/sensor-mapping.cfg
# Update this line with your NVMe name:
# storage_ssd[0]['temperature']: temperature_nvme_Composite_YOUR_NVME_NAME
```

#### 8. Set up systemd services

```bash
# aster-sysinfo service
cat > /etc/systemd/system/aster-sysinfo.service << 'EOF'
[Unit]
Description=AOOSTAR Sensor Provider
After=network.target

[Service]
ExecStart=/usr/local/bin/aster-sysinfo --refresh 5 -o /root/aoostar-rs/cfg/sensors/values.txt --temp-dir /root/aoostar-rs/cfg/sensors/
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# asterctl service
cat > /etc/systemd/system/asterctl.service << 'EOF'
[Unit]
Description=AOOSTAR Screen Control
After=aster-sysinfo.service

[Service]
WorkingDirectory=/root/aoostar-rs
ExecStart=/usr/local/bin/asterctl --config-dir /root/aoostar-rs/cfg --config monitor.json --sensor-path /root/aoostar-rs/cfg/sensors/values.txt --sensor-mapping /root/aoostar-rs/cfg/sensor-mapping.cfg
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# proxmox-sensors service
cat > /etc/systemd/system/proxmox-sensors.service << 'EOF'
[Unit]
Description=Proxmox Sensors for AOOSTAR LCD
After=network.target aster-sysinfo.service

[Service]
ExecStart=/bin/bash /root/aoostar-rs/proxmox-sensors.sh
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# webui service
cat > /etc/systemd/system/aoostar-webui.service << 'EOF'
[Unit]
Description=AOOSTAR Screen Web Editor
After=network.target

[Service]
ExecStart=/usr/bin/python3 /root/aoostar-rs/webui.py
Restart=always
WorkingDirectory=/root/aoostar-rs
Environment=PYTHONUNBUFFERED=1

[Install]
WantedBy=multi-user.target
EOF

# Enable and start all services
systemctl daemon-reload
systemctl enable aster-sysinfo asterctl proxmox-sensors aoostar-webui
systemctl start aster-sysinfo asterctl proxmox-sensors aoostar-webui
```

---

### Method 2 — Docker

#### Prerequisites
- Docker installed on Proxmox

#### 1. Clone this repository

```bash
git clone https://github.com/YOUR_USERNAME/aoostar-proxmox-lcd.git
cd aoostar-proxmox-lcd
```

#### 2. Build the Docker image

```bash
docker build -t aoostar-lcd:latest .
```
> ⚠️ This takes 10-15 minutes (Rust compilation inside Docker)

#### 3. Deploy with Docker Compose

```bash
docker compose up -d
```

#### Or deploy via Portainer

1. Build the image locally (step 2 above)
2. In Portainer → **Stacks** → **Add stack**
3. Paste the contents of `docker-compose.yml`
4. Click **Deploy the stack**

---

## 🌐 Web Editor

Access the visual editor at:
```
http://YOUR_PROXMOX_IP:8765
```

### Editor Features

| Feature | Description |
|---|---|
| Drag & drop | Move elements directly on the preview |
| Snap to grid | Auto-align on configurable grid |
| Color picker | Set text color per element |
| Image upload | Auto-resize to 960×376 |
| Live Preview | Show real sensor values on preview |
| Transition | Set panel switch duration in seconds |
| Export/Import | Save and restore JSON configuration |
| Ctrl+Z | Undo up to 30 actions |
| Add panel | Create new display panels |
| Labels | Click a label to apply it to selected element |

---

## 📐 Screen Specifications

| Property | Value |
|---|---|
| Resolution | 960 × 376 pixels |
| Interface | USB Virtual COM |
| Device | `/dev/ttyACM0` |
| Chipset | Winbond `0416:90a1` |
| Baud rate | 1,500,000 |

---

## 🔧 Troubleshooting

### Screen not detected
```bash
ls /dev/ttyACM0
lsusb | grep Winbond
```

### Values stuck at 98 (default)
```bash
# Check sensor mapping
cat /root/aoostar-rs/cfg/sensor-mapping.cfg
# Check available values
cat /root/aoostar-rs/cfg/sensors/values.txt | grep temperature
```

### apt 401 Unauthorized errors
You have the enterprise repository enabled without a subscription. Follow step 1 of the installation to fix this.

### Script tteck refuses to run (version detection bug)
The tteck script may not recognize Proxmox 9.x. Use the manual installation method described above.

### Must compile from source
The pre-built `asterctl` binary v0.2.0 does not support `--sensor-mapping`. Always compile from the [zehnm/aoostar-rs](https://github.com/zehnm/aoostar-rs) source.

---

## 📁 Project Structure

```
aoostar-proxmox-lcd/
├── Dockerfile                    # Docker build file
├── docker-compose.yml            # Docker Compose configuration
├── start.sh                      # Docker entrypoint script
├── webui.py                      # Web editor (Flask)
├── proxmox-sensors.sh            # Proxmox-specific metrics script
├── cfg/
│   ├── monitor.json              # Panel configuration
│   ├── sensor-mapping.cfg        # Sensor label mapping
│   ├── sensor-mapping-filter.cfg # Sensor filter rules
│   ├── default_1_index.jpg       # Panel 1 background
│   ├── default_1_hdd.jpg         # Panel 2 background
│   └── proxmox_panel.jpg         # Panel 3 background (generated)
└── README.md
```

---

## 🙏 Credits

- [zehnm/aoostar-rs](https://github.com/zehnm/aoostar-rs) — Original asterctl project for AOOSTAR LCD control
- Web editor and Proxmox integration developed with [Claude](https://claude.ai) (Anthropic)

---

## 📄 License

MIT License — feel free to use, modify and share!

---

## 🤝 Contributing

Pull requests welcome! If you have improvements, bug fixes, or support for other AOOSTAR models, feel free to contribute.

If this project helped you, give it a ⭐ on GitHub!
