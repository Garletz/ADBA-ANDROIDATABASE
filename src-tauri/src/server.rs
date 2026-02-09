//! REST API Server using Axum
//! 
//! Provides HTTP endpoints for database operations
//! Clients can connect via standard HTTP requests

use crate::error::AdbaError;
use crate::state::AppState;
use axum::{
    extract::{Json, Path, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};

/// Start the REST API server
pub async fn start_rest_server(state: Arc<AppState>) -> Result<u16, AdbaError> {
    // Try to bind to port 8080
    let port = 8080;
    let addr = format!("0.0.0.0:{}", port);
    
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| AdbaError::Server(format!("Failed to bind to {}: {}", addr, e)))?;
    
    let local_addr = listener.local_addr()
        .map_err(|e| AdbaError::Server(e.to_string()))?;
    let bound_port = local_addr.port();
    
    state.set_pg_port(bound_port); // Reusing pg_port field for API port
    
    info!("REST API server starting on {}", local_addr);
    
    // Configure CORS for LAN access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);
    
    // Build the router
    let app = Router::new()
        // Status endpoints
        .route("/api/status", get(get_status))
        .route("/api/info", get(get_connection_info))
        
        // Database management
        .route("/api/databases", get(list_databases))
        .route("/api/databases", post(create_database))
        .route("/api/databases/:name", get(get_database))
        .route("/api/databases/:name", delete(delete_database))
        
        // Query execution
        .route("/api/query", post(execute_query))
        
        // Pairing
        .route("/api/pair", post(validate_pairing))
        .route("/api/pairing-code", get(get_pairing_code))
        .route("/api/pairing-code", post(regenerate_pairing_code))
        
        .layer(cors)
        .with_state(state.clone());
    
    // Spawn the server
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("REST API server error: {}", e);
        }
    });
    
    Ok(bound_port)
}

// =============================================================================
// Request/Response types
// =============================================================================

#[derive(Debug, Deserialize)]
struct CreateDatabaseRequest {
    name: String,
    client_app: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    database: String,
    query: String,
    pairing_code: String,
}

#[derive(Debug, Deserialize)]
struct PairingRequest {
    pairing_code: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ApiResponse {
    fn ok<T: Serialize>(data: T) -> (StatusCode, Json<Self>) {
        let value = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
        (StatusCode::OK, Json(Self {
            success: true,
            data: Some(value),
            error: None,
        }))
    }
    
    fn created<T: Serialize>(data: T) -> (StatusCode, Json<Self>) {
        let value = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
        (StatusCode::CREATED, Json(Self {
            success: true,
            data: Some(value),
            error: None,
        }))
    }
    
    fn err(status: StatusCode, message: &str) -> (StatusCode, Json<Self>) {
        (status, Json(Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }))
    }
}

// =============================================================================
// Handlers
// =============================================================================

async fn get_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let status = state.get_status().await;
    ApiResponse::ok(status)
}

async fn get_connection_info(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let info = state.get_connection_info().await;
    ApiResponse::ok(info)
}

async fn list_databases(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.list_databases().await {
        Ok(dbs) => ApiResponse::ok(dbs),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn create_database(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDatabaseRequest>,
) -> impl IntoResponse {
    let client_app = payload.client_app.unwrap_or_else(|| "unknown".to_string());
    
    match state.db.create_database(&payload.name, &client_app).await {
        Ok(db) => ApiResponse::created(db),
        Err(e) => ApiResponse::err(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn get_database(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.db.get_database(&name).await {
        Ok(Some(db)) => ApiResponse::ok(db),
        Ok(None) => ApiResponse::err(StatusCode::NOT_FOUND, "Database not found"),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn delete_database(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_database(&name).await {
        Ok(()) => ApiResponse::ok(serde_json::json!({ "deleted": name })),
        Err(e) => ApiResponse::err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn execute_query(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueryRequest>,
) -> impl IntoResponse {
    // Validate pairing code
    if !state.validate_pairing_code(&payload.pairing_code) {
        return ApiResponse::err(StatusCode::UNAUTHORIZED, "Invalid pairing code");
    }
    
    match state.db.execute_query(&payload.database, &payload.query).await {
        Ok(result) => ApiResponse::ok(result),
        Err(e) => ApiResponse::err(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn validate_pairing(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PairingRequest>,
) -> impl IntoResponse {
    let valid = state.validate_pairing_code(&payload.pairing_code);
    ApiResponse::ok(serde_json::json!({ "valid": valid }))
}

async fn get_pairing_code(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let code = state.pairing_code.clone();
    ApiResponse::ok(serde_json::json!({ "pairing_code": code }))
}

async fn regenerate_pairing_code(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let new_code = state.regenerate_pairing_code();
    ApiResponse::ok(serde_json::json!({ "pairing_code": new_code }))
}
