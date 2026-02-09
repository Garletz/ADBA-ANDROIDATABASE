//! Application state management

use crate::database::{DatabaseEngine, DatabaseInfo};
use crate::error::AdbaError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU16, Ordering};
use uuid::Uuid;

/// Shared application state
pub struct AppState {
    pub db: DatabaseEngine,
    pub pairing_code: String,
    pairing_code_inner: RwLock<String>,
    pg_port: AtomicU16,
    active_connections: RwLock<Vec<ConnectionSession>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub running: bool,
    pub pg_port: u16,
    pub databases_count: usize,
    pub active_connections: usize,
    pub pairing_code: String,
    pub local_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub pairing_code: String,
    pub connection_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSession {
    pub id: String,
    pub client_app: String,
    pub database: String,
    pub connected_at: i64,
}

impl AppState {
    pub fn new(db: DatabaseEngine) -> Self {
        let pairing_code = generate_pairing_code();
        Self {
            db,
            pairing_code: pairing_code.clone(),
            pairing_code_inner: RwLock::new(pairing_code),
            pg_port: AtomicU16::new(5433),
            active_connections: RwLock::new(Vec::new()),
        }
    }
    
    pub fn set_pg_port(&self, port: u16) {
        self.pg_port.store(port, Ordering::SeqCst);
    }
    
    pub async fn get_status(&self) -> ServerStatus {
        let dbs = self.db.list_databases().await.unwrap_or_default();
        let connections = self.active_connections.read();
        let local_ip = get_local_ip();
        
        ServerStatus {
            running: true,
            pg_port: self.pg_port.load(Ordering::SeqCst),
            databases_count: dbs.len(),
            active_connections: connections.len(),
            pairing_code: self.pairing_code_inner.read().clone(),
            local_ip,
        }
    }
    
    pub async fn get_databases(&self) -> Result<Vec<DatabaseInfo>, AdbaError> {
        self.db.list_databases().await
    }
    
    pub async fn create_database(&self, name: &str, client_app: &str) -> Result<DatabaseInfo, AdbaError> {
        self.db.create_database(name, client_app).await
    }
    
    pub fn regenerate_pairing_code(&self) -> String {
        let new_code = generate_pairing_code();
        *self.pairing_code_inner.write() = new_code.clone();
        new_code
    }
    
    pub fn validate_pairing_code(&self, code: &str) -> bool {
        *self.pairing_code_inner.read() == code
    }
    
    pub fn add_connection(&self, session: ConnectionSession) {
        self.active_connections.write().push(session);
    }
    
    pub fn remove_connection(&self, id: &str) {
        self.active_connections.write().retain(|s| s.id != id);
    }
    
    pub async fn get_connection_info(&self) -> ConnectionInfo {
        let port = self.pg_port.load(Ordering::SeqCst);
        let host = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        let pairing_code = self.pairing_code_inner.read().clone();
        
        ConnectionInfo {
            connection_string: format!("postgresql://adba:{}@{}:{}/main", pairing_code, host, port),
            host,
            port,
            pairing_code,
        }
    }
}

/// Generate a 6-character alphanumeric pairing code
fn generate_pairing_code() -> String {
    let uuid = Uuid::new_v4();
    uuid.to_string()[..6].to_uppercase()
}

/// Get local IP address for LAN
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}
