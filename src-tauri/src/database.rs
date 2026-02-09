//! Database engine using SQLite (rusqlite)
//! 
//! Provides multi-tenant database management for client apps
//! Each client app gets its own SQLite database file
//! 
//! Note: rusqlite::Connection is not Sync, so we use tokio::sync::Mutex
//! and spawn_blocking for database operations

use crate::error::AdbaError;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use tracing::info;

/// Information about a database hosted in ADBA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub id: String,
    pub name: String,
    pub client_app: String,
    pub created_at: i64,
    pub size_bytes: u64,
    pub tables_count: usize,
    pub status: DatabaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseStatus {
    Active,
    Syncing,
    Offline,
    Error,
}

/// Main database engine managing multiple SQLite databases
/// Uses Arc<Mutex<>> for thread-safe access to SQLite connections
pub struct DatabaseEngine {
    data_dir: PathBuf,
}

// Manually implement Send + Sync since we handle synchronization ourselves
unsafe impl Send for DatabaseEngine {}
unsafe impl Sync for DatabaseEngine {}

impl DatabaseEngine {
    /// Create a new database engine
    pub async fn new() -> Result<Self, AdbaError> {
        let data_dir = get_data_directory();
        std::fs::create_dir_all(&data_dir)?;
        
        let metadata_path = data_dir.join("metadata.db");
        info!("Initializing metadata database at {:?}", metadata_path);
        
        // Initialize metadata in a blocking context

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(metadata_path)?;
            conn.execute(
                "CREATE TABLE IF NOT EXISTS databases (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    client_app TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )",
                [],
            )?;
            Ok::<_, rusqlite::Error>(())
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e| AdbaError::Database(e.to_string()))?;
        
        info!("Metadata database initialized successfully");
        
        Ok(Self { data_dir })
    }
    
    /// Create a new database for a client app
    pub async fn create_database(&self, name: &str, client_app: &str) -> Result<DatabaseInfo, AdbaError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono_timestamp();
        let db_filename = sanitize_name(name);
        let db_path = self.data_dir.join(format!("{}.db", db_filename));
        let metadata_path = self.data_dir.join("metadata.db");
        
        let name_owned = name.to_string();
        let client_app_owned = client_app.to_string();
        let id_owned = id.clone();
        
        tokio::task::spawn_blocking(move || {
            // Create the database file
            let _conn = Connection::open(&db_path)?;
            
            // Store metadata
            let meta_conn = Connection::open(&metadata_path)?;
            meta_conn.execute(
                "INSERT INTO databases (id, name, client_app, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id_owned, name_owned, client_app_owned, now],
            )?;
            
            Ok::<_, rusqlite::Error>(())
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e| AdbaError::Database(e.to_string()))?;
        
        let db_path_for_size = self.data_dir.join(format!("{}.db", db_filename));
        let info = DatabaseInfo {
            id,
            name: name.to_string(),
            client_app: client_app.to_string(),
            created_at: now,
            size_bytes: get_file_size(&db_path_for_size),
            tables_count: 0,
            status: DatabaseStatus::Active,
        };
        
        info!("Created database '{}' for app '{}'", name, client_app);
        
        Ok(info)
    }
    
    /// List all databases
    pub async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, AdbaError> {
        let metadata_path = self.data_dir.join("metadata.db");
        let data_dir = self.data_dir.clone();
        
        let databases = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&metadata_path)?;
            
            let mut stmt = conn.prepare(
                "SELECT id, name, client_app, created_at FROM databases ORDER BY created_at DESC"
            )?;
            
            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let client_app: String = row.get(2)?;
                let created_at: i64 = row.get(3)?;
                
                let db_path = data_dir.join(format!("{}.db", sanitize_name(&name)));
                let size_bytes = get_file_size(&db_path);
                let tables_count = get_table_count(&db_path);
                
                Ok(DatabaseInfo {
                    id,
                    name,
                    client_app,
                    created_at,
                    size_bytes,
                    tables_count,
                    status: DatabaseStatus::Active,
                })
            })?;
            
            let mut databases = Vec::new();
            for row in rows {
                if let Ok(db) = row {
                    databases.push(db);
                }
            }
            
            Ok::<_, rusqlite::Error>(databases)
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e| AdbaError::Database(e.to_string()))?;
        
        Ok(databases)
    }
    
    /// Get a specific database by name
    pub async fn get_database(&self, name: &str) -> Result<Option<DatabaseInfo>, AdbaError> {
        let metadata_path = self.data_dir.join("metadata.db");
        let data_dir = self.data_dir.clone();
        let name_owned = name.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&metadata_path)?;
            
            let mut stmt = conn.prepare(
                "SELECT id, name, client_app, created_at FROM databases WHERE name = ?1"
            )?;
            
            let result = stmt.query_row(params![name_owned], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let client_app: String = row.get(2)?;
                let created_at: i64 = row.get(3)?;
                
                let db_path = data_dir.join(format!("{}.db", sanitize_name(&name)));
                
                Ok(DatabaseInfo {
                    id,
                    name,
                    client_app,
                    created_at,
                    size_bytes: get_file_size(&db_path),
                    tables_count: get_table_count(&db_path),
                    status: DatabaseStatus::Active,
                })
            });
            
            match result {
                Ok(db) => Ok(Some(db)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e| AdbaError::Database(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Delete a database
    pub async fn delete_database(&self, name: &str) -> Result<(), AdbaError> {
        let metadata_path = self.data_dir.join("metadata.db");
        let db_path = self.data_dir.join(format!("{}.db", sanitize_name(name)));
        let name_owned = name.to_string();
        
        tokio::task::spawn_blocking(move || {
            // Remove from metadata
            let conn = Connection::open(&metadata_path)?;
            conn.execute("DELETE FROM databases WHERE name = ?1", params![name_owned])?;
            
            // Delete the database file
            if db_path.exists() {
                std::fs::remove_file(&db_path)?;
            }
            
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e| AdbaError::Database(e.to_string()))?;
        
        info!("Deleted database '{}'", name);
        
        Ok(())
    }
    
    /// Execute a raw SQL query on a specific database
    pub async fn execute_query(
        &self,
        database: &str,
        query: &str
    ) -> Result<serde_json::Value, AdbaError> {
        let db_path = self.data_dir.join(format!("{}.db", sanitize_name(database)));
        let query_owned = query.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            
            let query_upper = query_owned.trim().to_uppercase();
            
            if query_upper.starts_with("SELECT") {
                // Return results as JSON
                let mut stmt = conn.prepare(&query_owned)?;
                
                let column_names: Vec<String> = stmt.column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                
                let mut rows_json = Vec::new();
                let mut rows = stmt.query([])?;
                
                while let Some(row) = rows.next()? {
                    let mut obj = serde_json::Map::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value: rusqlite::Result<String> = row.get(i);
                        match value {
                            Ok(v) => { obj.insert(name.clone(), serde_json::Value::String(v)); }
                            Err(_) => {
                                // Try as integer
                                if let Ok(v) = row.get::<_, i64>(i) {
                                    obj.insert(name.clone(), serde_json::json!(v));
                                } else if let Ok(v) = row.get::<_, f64>(i) {
                                    obj.insert(name.clone(), serde_json::json!(v));
                                } else {
                                    obj.insert(name.clone(), serde_json::Value::Null);
                                }
                            }
                        }
                    }
                    rows_json.push(serde_json::Value::Object(obj));
                }
                
                Ok(serde_json::json!(rows_json))
            } else {
                // Execute non-SELECT query
                let affected = conn.execute(&query_owned, [])?;
                Ok(serde_json::json!({
                    "affected_rows": affected
                }))
            }
        }).await
        .map_err(|e| AdbaError::Database(e.to_string()))?
        .map_err(|e: rusqlite::Error| AdbaError::Database(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Get the data directory
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

/// Get the data directory for storing databases
fn get_data_directory() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // Android internal storage
        PathBuf::from("/data/data/com.administrateur.adba/databases")
    }
    
    #[cfg(not(target_os = "android"))]
    {
        // Desktop: use local directory for development
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".adba").join("data")
    }
}

/// Sanitize a name for use as filename
fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

/// Get current timestamp in milliseconds
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Get file size in bytes
fn get_file_size(path: &PathBuf) -> u64 {
    std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0)
}

/// Count tables in a SQLite database
fn get_table_count(path: &PathBuf) -> usize {
    if let Ok(conn) = Connection::open(path) {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'"
        ) {
            if let Ok(count) = stmt.query_row([], |row| row.get::<_, i32>(0)) {
                return count as usize;
            }
        }
    }
    0
}
