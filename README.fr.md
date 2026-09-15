<p align="center">
  <img src="docs/assets/logo.svg" width="170" alt="Logo AooScope pour écran LCD AOOSTAR WTR MAX">
</p>

<h1 align="center">AooScope — Dashboard LCD pour AOOSTAR WTR MAX</h1>

<p align="center"><strong>Le dashboard Linux open source pour l’écran de monitoring de l’AOOSTAR WTR MAX.</strong><br>
Transforme le LCD frontal 960×376 en écran Proxmox, stockage, hardware et média réellement utile.</p>

<p align="center">🇬🇧 <a href="README.md">English README</a></p>

<p align="center">
  <img src="docs/assets/screenshots/studio.png" width="100%" alt="Studio visuel AooScope pour écran LCD AOOSTAR WTR MAX">
</p>

AooScope est un studio de contrôle communautaire pensé pour le **LCD de l’AOOSTAR WTR MAX**. À la place d’un petit mur de chiffres : grandes valeurs, anneaux et barres lisibles, santé des disques, métriques Proxmox, affiches média et ETA humains comme **READY IN 8 MIN**.

La cible principale est l’**AOOSTAR WTR MAX 8845HS / WTR MAX NAS** sous Linux, Proxmox ou tout OS homelab capable de faire tourner Docker. D’autres machines AOOSTAR utilisant le même protocole LCD série supporté peuvent également fonctionner.

## Pourquoi AooScope

- **Pensé pour le WTR MAX** — interface et pages conçues autour du vrai espace 960×376 de l’écran.
- **Studio visuel** — glisser, redimensionner et connecter des métriques par noms lisibles ; les IDs techniques restent dans Advanced.
- **Stockage adaptatif** — détection des disques, used/total, barre de capacité, température, SMART et pagination automatique.
- **Média lisible d’un coup d’œil** — Jellyfin/Silo + arrivées Radarr/Sonarr/qBittorrent avec affiche, progression et temps restant.
- **Proxmox + hardware** — CPU, RAM, VM/LXC, thermals Linux/sysfs et activité Radeon sans deuxième daemon sur l’hôte.
- **Contrôles d’écran sûrs** — power supporté, luminance logicielle, intervalle du carrousel et planning sans opcode matériel non documenté.
- **Un seul petit conteneur** — backend Rust/Axum avec l’UI Svelte embarquée, sans runtime Python ou Node sur la machine cible.

## À quoi ça ressemble

<p align="center">
  <img src="docs/assets/screenshots/media.png" width="49%" alt="Dashboard média AooScope AOOSTAR WTR MAX avec Radarr qBittorrent affiche progression et ETA">
  <img src="docs/assets/screenshots/display.png" width="49%" alt="Contrôles power luminance et carrousel du LCD AOOSTAR WTR MAX dans AooScope">
</p>

Les captures utilisent des données de démonstration génériques, mais proviennent de la véritable interface de l’application.

## Studio visuel de pages

L’éditeur canvas-first garde le LCD visible pendant le travail. Choisis une métrique par son nom humain et AooScope sélectionne une représentation adaptée ; valeur, barre, anneau, jauge ou badge restent interchangeables lorsque cela a du sens. Les templates **Semi rings**, **Vertical bars**, **Horizontal bars**, **Media** et **Storage cards** créent des couches ordinaires entièrement modifiables.

Les pages Storage sont générées depuis l’inventaire réel et paginées jusqu’à six disques par page. Une régénération explicite n’écrase pas les pages que tu as personnalisées.

Les médias et animations de splash passent par la même pipeline bornée. Les GIF affichent de vraies frames sur le LCD, les posters sont mis en cache localement et les téléchargements d’illustrations externes sont restreints à des origines sûres.

Les modifications restent en brouillon jusqu’à **Apply to LCD**. Le système de révision évite qu’un planning de luminosité ou un refresh publie accidentellement un draft.

## Installation rapide

Prérequis : Linux + Docker/Compose, écran AOOSTAR supporté exposé en série (souvent `/dev/ttyACM0`) et droits Docker sur ce périphérique.

```bash
mkdir -p aooscope/data
cd aooscope
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/compose.yaml
curl -fsSLO https://raw.githubusercontent.com/GodsQuantum/AooScope/main/.env.example
cp .env.example .env

docker compose up -d
```

Interface : `http://127.0.0.1:8765` par défaut. Pour le LAN, configure `AOOSCOPE_BIND_ADDRESS` vers une adresse de confiance.

Image : `ghcr.io/godsquantum/aooscope:latest`.

## Comportement du LCD WTR MAX

```text
Splash → Home → Storage → Compute
```

Une lecture ou un téléchargement peut temporairement prendre la priorité, puis rendre la main au carrousel : `24 MIN LEFT`, `READY IN 9 MIN`, `JUST LANDED`, `MEDIA OFFLINE`.

> **Luminosité :** aucune commande native documentée de rétroéclairage WTR MAX n’est actuellement exposée par `aoostar-rs`. Le curseur AooScope agit donc sur les pixels rendus, pas sur la puissance physique du backlight.

## Providers

L’Admin configure **Proxmox VE, Beszel, Jellyfin, Silo, Radarr, Sonarr, qBittorrent, Immich et Ollama**. Proxmox, Jellyfin, Silo, Radarr, Sonarr et qBittorrent alimentent déjà l’affichage runtime.

Voir [`docs/providers.md`](docs/providers.md) et [`docs/deployment.md`](docs/deployment.md).

## Sécurité

L’Admin n’intègre pas d’authentification. Le Compose public écoute donc uniquement sur `127.0.0.1` par défaut. Pour un accès distant, utilise un VPN ou un reverse proxy authentifié.

Les secrets providers sont stockés séparément et ne sont jamais renvoyés par l’API publique. Les posters sont téléchargés avec une taille bornée, sans redirects, et les credentials providers ne sont jamais transmis aux CDN d’illustrations externes.

Voir [`SECURITY.md`](SECURITY.md).

## Crédits

- [`xavtb78/aoostar-proxmox-lcd`](https://github.com/xavtb78/aoostar-proxmox-lcd)
- [`zehnm/aoostar-rs`](https://github.com/zehnm/aoostar-rs)

AooScope est un logiciel communautaire indépendant, non affilié et non approuvé par AOOSTAR.
