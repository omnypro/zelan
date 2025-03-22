use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::adapters::ServiceAdapterStatus;
use crate::error::Result;
use crate::service::StreamService;

pub use self::ApiServer;

/// API server state containing a reference to the stream service
#[derive(Clone)]
struct ApiState {
    service: Arc<RwLock<StreamService>>,
}

/// Response data for API endpoints
#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Request data for starting/stopping adapters
#[derive(Debug, Deserialize)]
struct AdapterRequest {
    id: String,
}

/// Request data for configuring adapters
#[derive(Debug, Deserialize)]
struct ConfigRequest {
    config: Value,
}

/// Implementation of the REST API server
pub struct ApiServer {
    service: Arc<RwLock<StreamService>>,
    port: u16,
}

impl ApiServer {
    /// Create a new API server with the given service and port
    pub fn new(service: StreamService, port: u16) -> Self {
        ApiServer {
            service: Arc::new(RwLock::new(service)),
            port,
        }
    }
    
    /// Start the API server
    pub async fn start(self) -> Result<()> {
        let state = ApiState {
            service: self.service,
        };
        
        // Define API routes
        let app = Router::new()
            .route("/api/status", get(get_status))
            .route("/api/adapters", get(get_adapters))
            .route("/api/adapters/:id", get(get_adapter))
            .route("/api/adapters/:id/connect", post(connect_adapter))
            .route("/api/adapters/:id/disconnect", post(disconnect_adapter))
            .route("/api/adapters/:id/configure", post(configure_adapter))
            .with_state(state);
        
        // Start the server
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        println!("Starting API server on {}", addr);
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .map_err(|e| crate::error::Error::internal(format!("API server error: {}", e)))?;
        
        Ok(())
    }
}

/// Get overall service status
async fn get_status(
    State(state): State<ApiState>,
) -> (StatusCode, Json<ApiResponse<HashMap<String, bool>>>) {
    let service = state.service.read().await;
    let service_running = *service.running.read().await;
    
    let mut data = HashMap::new();
    data.insert("running".to_string(), service_running);
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }),
    )
}

/// Get all adapter statuses
async fn get_adapters(
    State(state): State<ApiState>,
) -> (StatusCode, Json<ApiResponse<HashMap<String, ServiceAdapterStatus>>>) {
    let service = state.service.read().await;
    let adapters = service.get_all_adapter_statuses().await;
    
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: Some(adapters),
            error: None,
        }),
    )
}

/// Get a specific adapter's status
async fn get_adapter(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ServiceAdapterStatus>>) {
    let service = state.service.read().await;
    
    match service.get_adapter_status(&id).await {
        Ok(status) => (
            StatusCode::OK,
            Json(ApiResponse {
                success: true,
                data: Some(status),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Connect a specific adapter
async fn connect_adapter(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ServiceAdapterStatus>>) {
    let service = state.service.read().await;
    
    match service.connect_adapter(&id).await {
        Ok(()) => {
            match service.get_adapter_status(&id).await {
                Ok(status) => (
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(status),
                        error: None,
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Disconnect a specific adapter
async fn disconnect_adapter(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ServiceAdapterStatus>>) {
    let service = state.service.read().await;
    
    match service.disconnect_adapter(&id).await {
        Ok(()) => {
            match service.get_adapter_status(&id).await {
                Ok(status) => (
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(status),
                        error: None,
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Configure a specific adapter
async fn configure_adapter(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(request): Json<ConfigRequest>,
) -> (StatusCode, Json<ApiResponse<ServiceAdapterStatus>>) {
    let service = state.service.read().await;
    
    // Get the adapter
    let adapters = service.adapters.read().await;
    let adapter = match adapters.get(&id) {
        Some(adapter) => adapter,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Adapter with ID '{}' not found", id)),
                }),
            );
        }
    };
    
    // Configure the adapter
    match adapter.configure(request.config).await {
        Ok(()) => {
            // Get the updated status
            match adapter.status().await {
                Ok(status) => (
                    StatusCode::OK,
                    Json(ApiResponse {
                        success: true,
                        data: Some(status),
                        error: None,
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}