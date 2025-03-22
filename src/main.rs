use tracing::{info, debug};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod adapters;
mod api;
mod auth;
mod config;
mod core;
mod error;
mod websocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file if it exists
    let env_file_path = match dotenvy::dotenv() {
        Ok(path) => Some(path),
        Err(_) => None,
    };

    // Initialize the tracing subscriber for structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to info level if RUST_LOG is not set
            if cfg!(debug_assertions) {
                // More verbose in debug mode
                "zelan=info,zelan::core::event_bus=trace,zelan::adapters::obs=debug,warn"
                    .into()
            } else {
                // Less verbose in release mode
                "zelan=info,zelan::adapters::obs=info,warn".into()
            }
        }))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    info!("Zelan API server starting");

    // Log environment loading after logger is initialized
    match env_file_path {
        Some(path) => info!("Loaded environment variables from {}", path.display()),
        None => debug!("No .env file found. Using existing environment variables."),
    };

    // Load configuration
    let config = config::load_config().await?;
    
    // Initialize the core service
    let service = core::StreamService::new(config.clone());
    
    // Start the WebSocket server
    service.start_websocket_server().await?;
    
    // Start the API server
    let api_server = api::start_server(config.clone(), service.clone()).await?;
    
    info!("Server started successfully");
    info!("Press Ctrl+C to stop the server");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping server...");
    
    // Graceful shutdown
    api_server.shutdown().await;
    
    info!("Server shutdown complete");
    Ok(())
}