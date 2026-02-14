//! ADBA - Android Database Application
//! 
//! Main library providing:
//! - SurrealDB embedded database engine
//! - PostgreSQL wire protocol server (via pgwire)
//! - mDNS service discovery for LAN visibility
//! - Tauri commands for frontend communication

mod database;
mod server;
mod discovery;
mod state;
mod error;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;

/// Initialize the ADBA backend services
async fn init_services(_app_handle: tauri::AppHandle) -> Result<Arc<AppState>, error::AdbaError> {
    info!("Initializing ADBA services...");
    
    // Initialize database engine
    let db = database::DatabaseEngine::new().await?;
    
    // Create app state
    let state = Arc::new(AppState::new(db));
    
    // Start REST API server
    let api_port = server::start_rest_server(state.clone()).await?;
    info!("REST API server listening on port {}", api_port);
    
    // Register mDNS service for LAN discovery
    discovery::register_service(api_port, &state.pairing_code)?;
    info!("Service registered on LAN with pairing code: {}", state.pairing_code);
    
    Ok(state)
}

// ============================================================================
// Tauri Commands - Called from Frontend
// ============================================================================

/// Get current server status
#[tauri::command]
async fn get_status(state: tauri::State<'_, Arc<AppState>>) -> Result<state::ServerStatus, String> {
    Ok(state.get_status().await)
}

/// Get list of connected databases
#[tauri::command]
async fn get_databases(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<database::DatabaseInfo>, String> {
    state.get_databases().await.map_err(|e| e.to_string())
}

/// Create a new database namespace for a client app
#[tauri::command]
async fn create_database(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    client_app: String
) -> Result<database::DatabaseInfo, String> {
    state.create_database(&name, &client_app).await.map_err(|e| e.to_string())
}

/// Get pairing code for client connection
#[tauri::command]
fn get_pairing_code(state: tauri::State<'_, Arc<AppState>>) -> String {
    state.pairing_code.clone()
}

/// Regenerate pairing code
#[tauri::command]
fn regenerate_pairing_code(state: tauri::State<'_, Arc<AppState>>) -> String {
    state.regenerate_pairing_code()
}

/// Get connection info for clients
#[tauri::command]
async fn get_connection_info(state: tauri::State<'_, Arc<AppState>>) -> Result<state::ConnectionInfo, String> {
    Ok(state.get_connection_info().await)
}

// ============================================================================
// Tauri Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Block on async initialization to ensure services are ready
            tauri::async_runtime::block_on(async move {
                match init_services(handle.clone()).await {
                    Ok(state) => {
                        handle.manage(state);
                        info!("ADBA services initialized successfully");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize services: {}", e);
                        Err(Box::new(e) as Box<dyn std::error::Error>)
                    }
                }
            })
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_databases,
            create_database,
            get_pairing_code,
            regenerate_pairing_code,
            get_connection_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
