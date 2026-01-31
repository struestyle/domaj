# 🐳 Domaj - Docker Mise à Jour

Système de Maintenance en Condition Opérationnelle (MCO) pour instances Docker.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)
![Docker](https://img.shields.io/badge/docker-compose-blue.svg)

## 📋 Fonctionnalités

- **Dashboard** : Vue d'ensemble de tous vos conteneurs Docker
- **Multi-serveurs** : Surveillez plusieurs serveurs depuis une interface unique
- **Détection de mises à jour** :
  - Comparaison du digest (même tag mis à jour sur le registre)
  - Comparaison avec le tag `latest`
- **Notifications par email** : Rapports périodiques configurables
- **Support multi-registres** : Docker Hub, Quay.io, GHCR

## 🚀 Démarrage rapide

### Prérequis

- Docker et Docker Compose
- (Optionnel) Rust 1.75+ pour le développement

### Installation

1. **Clonez le repository** :
```bash
git clone https://github.com/votre-user/domaj.git
cd domaj
```

2. **Configurez l'environnement** :
```bash
cp .env.example .env
# Éditez .env avec vos paramètres
```

3. **Lancez les services** :
```bash
docker compose up -d
```

4. **Accédez à l'interface** :
   - Frontend : http://localhost:8080
   - API : http://localhost:3000

## 🖥️ Déploiement de l'agent

Sur chaque serveur à surveiller, déployez l'agent Domaj :

```bash
docker run -d \
  --name domaj-agent \
  -p 3001:3001 \
  -v /var/run/docker.sock:/var/run/docker.sock:ro \
  -e API_KEY=votre-cle-api \
  domaj-agent:latest
```

Puis ajoutez le serveur via l'interface web.

## ⚙️ Configuration

| Variable | Description | Défaut |
|----------|-------------|--------|
| `SCAN_INTERVAL` | Fréquence de scan (cron) | `0 0 * * *` (quotidien) |
| `API_SECRET` | Secret pour l'auth des agents | - |
| `SMTP_HOST` | Serveur SMTP | - |
| `SMTP_PORT` | Port SMTP | `587` |
| `SMTP_USER` | Utilisateur SMTP | - |
| `SMTP_PASSWORD` | Mot de passe SMTP | - |
| `SMTP_FROM` | Adresse expéditeur | - |
| `NOTIFY_EMAILS` | Destinataires (virgule-séparé) | - |

### Exemples d'intervalles de scan

```bash
# Toutes les heures
SCAN_INTERVAL="0 * * * *"

# Tous les jours à 6h
SCAN_INTERVAL="0 6 * * *"

# Toutes les 6 heures
SCAN_INTERVAL="0 */6 * * *"
```

## 🛠️ Développement

### Structure du projet

```
domaj/
├── docker-compose.yml    # Orchestration
├── domaj-server/         # Backend Rust (API + scheduler)
├── domaj-agent/          # Agent Docker distant
└── domaj-web/            # Frontend SvelteKit
```

### Lancer en développement

```bash
# Backend
cd domaj-server
cargo run

# Agent  
cd domaj-agent
cargo run

# Frontend
cd domaj-web
npm install
npm run dev
```

## 📡 API Endpoints

| Méthode | Endpoint | Description |
|---------|----------|-------------|
| `GET` | `/api/servers` | Liste des serveurs |
| `POST` | `/api/servers` | Ajouter un serveur |
| `DELETE` | `/api/servers/:id` | Supprimer un serveur |
| `GET` | `/api/containers` | Tous les conteneurs |
| `GET` | `/api/updates` | Mises à jour disponibles |
| `POST` | `/api/scan` | Déclencher un scan |

## 🗺️ Roadmap

Voir [roadmap.txt](./roadmap.txt) pour les fonctionnalités planifiées :
- Notifications Telegram/Discord
- Support des registres privés
- Mises à jour automatiques
- Authentification OAuth2
- Scan CVE

## 📝 Licence

MIT License - voir [LICENSE](./LICENSE)
# domaj
