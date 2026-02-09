# ADBA - Android Database Application

Application Android servant de serveur de base de données local, accessible sur le réseau LAN.

## 🚀 Développement

### Prérequis locaux
- Node.js 18+
- Rust (rustup)
- **Pas besoin d'Android Studio** - les builds se font via GitHub Actions

### Commandes

```bash
# Installation des dépendances
npm install

# Développement desktop (pour tester la logique)
npm run tauri dev

# Le build Android se fait automatiquement sur GitHub (voir ci-dessous)
```

## 📱 Build Android (Cloud)

Les APK sont compilés automatiquement via **GitHub Actions** :

1. **Push** votre code sur GitHub (branch `main` ou `master`)
2. Le workflow se lance automatiquement
3. Téléchargez l'APK depuis l'onglet **Actions** → **Artifacts**

Ou lancez manuellement : **Actions** → **Build Android APK** → **Run workflow**

## 🏗️ Architecture

```
ADBA/
├── src/                  # Frontend React
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs        # Entry point Tauri
│   │   ├── database.rs   # SQLite engine
│   │   ├── server.rs     # REST API (axum)
│   │   ├── discovery.rs  # mDNS LAN
│   │   ├── state.rs      # App state
│   │   └── error.rs      # Error types
│   └── Cargo.toml
└── .github/workflows/    # CI/CD
```

## 📡 API REST

| Endpoint | Méthode | Description |
|----------|---------|-------------|
| `/api/status` | GET | État du serveur |
| `/api/databases` | GET/POST | Liste/Créer DB |
| `/api/query` | POST | Exécuter SQL |
| `/api/pairing-code` | GET/POST | Code d'appairage |
