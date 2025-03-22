mod adapters;
mod api;
mod auth;
mod core;
mod error;
mod service;
mod websocket;

use std::env;
use tokio::signal;
use tracing_subscriber::fmt::format::FmtSpan;

use api::ApiServer;
use service::StreamService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("zelan_api=info")
        .with_span_events(FmtSpan::CLOSE)
        .init();
    
    // Read configuration from environment variables
    let websocket_port = env::var("WEBSOCKET_PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(8080);
    
    let api_port = env::var("API_PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(3000);
    
    tracing::info!("Starting Zelan API service...");
    
    // Create and initialize the stream service with all adapters
    let mut service = StreamService::new()
        .with_all_adapters()
        .await?;
    
    // Start the stream service
    service.start(Some(websocket_port)).await?;
    tracing::info!("Service started successfully on WebSocket port {}", websocket_port);
    
    // Create and start the API server
    let api_server = ApiServer::new(service, api_port);
    
    // Run the API server in a separate task
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api_server.start().await {
            tracing::error!("API server error: {:?}", e);
        }
    });
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Shutdown signal received, stopping service gracefully...");
        }
        Err(err) => {
            tracing::error!("Error waiting for shutdown signal: {}", err);
        }
    }
    
    // Cancel API server task
    api_handle.abort();
    
    tracing::info!("Service stopped successfully");
    Ok(())
}