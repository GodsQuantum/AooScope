<p align="center">
  <img src="docs/assets/logo.svg" width="170" alt="Logo AooScope">
</p>

<h1 align="center">AooScope</h1>

<p align="center"><strong>Un tableau de bord lisible d’un coup d’œil pour les écrans LCD AOOSTAR.</strong></p>

<p align="center">🇬🇧 <a href="README.md">English README</a></p>

---

AooScope transforme le petit écran AOOSTAR en véritable écran d’état serveur : gros chiffres, jauges compréhensibles à distance, santé du stockage, charge CPU/GPU et pages média événementielles.

Le projet est un fork puis une réécriture importante de `xavtb78/aoostar-proxmox-lcd`. `zehnm/aoostar-rs` reste le moteur bas niveau utilisé pour parler à l’écran.

## ✨ Points forts

- **Lisible de loin** — gros chiffres, couleurs sémantiques et jauges adaptées au 960×376.
- **Télémétrie matérielle** — températures Linux/sysfs, activité Radeon et mémoire GPU partagée.
- **Proxmox** — CPU, RAM, VM/LXC, stockage, ZFS et SMART via l’API native read-only.
- **Pages média** — Jellyfin/Silo + Radarr/qBittorrent avec affiche, progression et ETA.
- **Admin Web** — réglage de l’écran et connexion des providers par URL ou `IP:port`.
- **Secrets séparés** — credentials stockés hors des réglages publics et jamais renvoyés par l’API.
- **Luminosité logicielle planifiée** — par exemple `100%` en journée et `70%` de `22:00` à `08:00`.
- **Docker uniquement** — aucun daemon AooScope à installer directement sur Proxmox.

> **Luminosité :** AooScope ne connaît actuellement aucune commande native documentée de rétroéclairage WTR MAX. Le curseur agit sur la luminance des pixels rendus, pas sur la puissance physique du backlight.

## 🚀 Installation rapide

```bash
mkdir -p aooscope/data
cd aooscope
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/compose.yaml
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/.env.example
cp .env.example .env

docker compose up -d
```

Image : `ghcr.io/godsquantum/aooscope:latest`

L’interface est accessible sur `http://127.0.0.1:8765` par défaut. Pour le LAN, change `AOOSCOPE_BIND_ADDRESS` vers une adresse de confiance.

## 🖥️ Carrousel

```text
Logo → Home → Storage → Compute
```

Une lecture ou un téléchargement média peut prendre temporairement la priorité : `PLAYING 42%`, `READY IN 9 MIN`, `JUST LANDED`. A la fin de l’événement, retour automatique au carrousel normal.

## 🔌 Providers

L’Admin sait configurer Proxmox VE, Beszel, Jellyfin, Silo, Radarr, Sonarr, qBittorrent, Immich et Ollama. Proxmox/Jellyfin/Silo/Radarr/qBittorrent alimentent déjà l’affichage runtime.

Voir [`docs/providers.md`](docs/providers.md) et [`docs/deployment.md`](docs/deployment.md).

## 🔐 Sécurité

L’Admin n’intègre pas d’authentification. Le compose public écoute donc uniquement sur `127.0.0.1` par défaut. Pour un accès distant, utilise un VPN ou reverse proxy authentifié.

Voir [`SECURITY.md`](SECURITY.md).

## 🙏 Crédits

- [`xavtb78/aoostar-proxmox-lcd`](https://github.com/xavtb78/aoostar-proxmox-lcd)
- [`zehnm/aoostar-rs`](https://github.com/zehnm/aoostar-rs)

AooScope est indépendant et non affilié à AOOSTAR.
