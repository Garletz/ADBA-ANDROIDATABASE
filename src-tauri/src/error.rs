//! Error types for ADBA

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdbaError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Discovery error: {0}")]
    Discovery(String),
    
    #[error("Authentication failed: {0}")]
    Auth(String),
    
    #[error("Database not found: {0}")]
    NotFound(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<rusqlite::Error> for AdbaError {
    fn from(err: rusqlite::Error) -> Self {
        AdbaError::Database(err.to_string())
    }
}
