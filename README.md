<p align="center">
  <img src="src-tauri/icons/logo.png" width="180" alt="ADBA Logo">
</p>

<h1 align="center">ADBA</h1>
<h3 align="center">Android Database Application</h3>

<p align="center">
  <strong>Turn your Android phone into a local database server</strong>
</p>

<p align="center">
  <a href="#concept">Concept</a> |
  <a href="#how-it-works">How it Works</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#api">API</a>
</p>

---

## Concept

```mermaid
flowchart TB
    subgraph Phone["YOUR ANDROID PHONE"]
        ADBA["ADBA App"]
        DB[(Database)]
        ADBA --> DB
    end
    
    subgraph Same["SAME DEVICE"]
        App1["App 1"]
        App2["App 2"]
    end
    
    subgraph Network["SAME WIFI NETWORK"]
        Laptop["Laptop"]
        Tablet["Tablet"]
        OtherPhone["Other Phone"]
    end
    
    App1 -.->|localhost| ADBA
    App2 -.->|localhost| ADBA
    Laptop -.->|WiFi| ADBA
    Tablet -.->|WiFi| ADBA
    OtherPhone -.->|WiFi| ADBA
```

> **One phone = One database server**  
> Any app (local or on network) can store and query data

---

## How it Works

```mermaid
flowchart LR
    subgraph Clients["Any Client"]
        C1["Local App"]
        C2["PC Browser"]
        C3["Other Device"]
    end
    
    subgraph ADBA["ADBA Server"]
        API["REST API :8080"]
        SQLite[(SQLite)]
        mDNS["Auto-Discovery"]
    end
    
    C1 -->|HTTP| API
    C2 -->|HTTP| API
    C3 -->|HTTP| API
    API --> SQLite
    mDNS -.->|Broadcast| Clients
```

---

## Architecture

```mermaid
flowchart TB
    subgraph Frontend["React Dashboard"]
        UI["Status and Controls"]
    end
    
    subgraph Backend["Rust Backend"]
        Server["Axum REST"]
        DB["SQLite Engine"]
        Discovery["LAN Discovery"]
    end
    
    UI --> Server
    Server --> DB
    Server --> Discovery
```

---

## Quick Start

```bash
# Clone
git clone https://github.com/Garletz/ADBA-ANDROIDATABASE.git
cd ADBA-ANDROIDATABASE

# Install
npm install

# Dev (desktop)
npm run tauri dev
```

### Android APK
> Built automatically via GitHub Actions  
> Download from [Actions > Artifacts](../../actions)

---

## API

| Endpoint | Method | Description |
|:---------|:------:|:------------|
| `/api/status` | GET | Server status |
| `/api/databases` | GET | List all DBs |
| `/api/databases` | POST | Create DB |
| `/api/query` | POST | Execute SQL |
| `/api/pairing-code` | GET | Get connection code |

### Example

```bash
# Create database
curl -X POST http://PHONE_IP:8080/api/databases \
  -d '{"name": "myapp", "client_app": "MyApp"}'

# Query
curl -X POST http://PHONE_IP:8080/api/query \
  -d '{"database": "myapp", "query": "SELECT * FROM users", "pairing_code": "XXXX"}'
```

---

## Tech Stack

| Component | Technology |
|:----------|:-----------|
| Backend | Rust + Tauri |
| Database | SQLite (rusqlite) |
| API | Axum |
| Frontend | React + TypeScript |
| Discovery | mDNS |

---

<p align="center">
  <sub>Made for offline-first apps</sub>
</p>
