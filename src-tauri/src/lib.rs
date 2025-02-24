use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Router,
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{accept_async, tungstenite::Message};

// Re-export Adapters
pub mod adapters;
pub mod plugin;

// Constants for event bus configuration
const EVENT_BUS_CAPACITY: usize = 1000;
const RECONNECT_DELAY_MS: u64 = 5000;
const WS_PORT: u16 = 9000;

/// Standardized event structure for all events flowing through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Source service that generated the event (e.g., "obs", "twitch")
    source: String,
    /// Type of event (e.g., "scene.changed", "chat.message")
    event_type: String,
    /// Arbitrary JSON payload with event details
    payload: serde_json::Value,
    /// Timestamp when the event was created
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl StreamEvent {
    pub fn new(source: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get the source of this event
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Get the payload
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}

/// Central event bus for distributing events from adapters to subscribers
pub struct EventBus {
    sender: broadcast::Sender<StreamEvent>,
    buffer_size: usize,
    stats: Arc<RwLock<EventBusStats>>,
}

/// Statistics about event bus activity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventBusStats {
    events_published: u64,
    events_dropped: u64,
    source_counts: HashMap<String, u64>,
    type_counts: HashMap<String, u64>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            buffer_size: capacity,
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Get a subscriber to receive events
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: StreamEvent) -> Result<usize> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.events_published += 1;
            
            *stats.source_counts.entry(event.source.clone()).or_insert(0) += 1;
            *stats.type_counts.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        // Send the event
        match self.sender.send(event.clone()) {
            Ok(receivers) => Ok(receivers),
            Err(err) => {
                // If error indicates no subscribers, just record the statistic but don't treat as error
                if err.to_string().contains("channel closed") || err.to_string().contains("no receivers") {
                    let mut stats = self.stats.write().await;
                    stats.events_dropped += 1;
                    println!("No receivers for event {}.{}, event dropped", event.source(), event.event_type());
                    Ok(0) // Return 0 receivers instead of an error
                } else {
                    let mut stats = self.stats.write().await;
                    stats.events_dropped += 1;
                    Err(anyhow!("Failed to publish event: {}", err))
                }
            }
        }
    }

    /// Get current event bus statistics
    pub async fn get_stats(&self) -> EventBusStats {
        self.stats.read().await.clone()
    }

    /// Reset all statistics counters
    pub async fn reset_stats(&self) {
        *self.stats.write().await = EventBusStats::default();
    }

    /// Get the configured capacity of the event bus
    pub fn capacity(&self) -> usize {
        self.buffer_size
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            buffer_size: self.buffer_size,
            stats: Arc::clone(&self.stats),
        }
    }
}

/// Trait for service adapters that can connect to external services
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    /// Connect to the service
    async fn connect(&self) -> Result<()>;
    
    /// Disconnect from the service
    async fn disconnect(&self) -> Result<()>;
    
    /// Check if the adapter is currently connected
    fn is_connected(&self) -> bool;
    
    /// Get the adapter's name
    fn get_name(&self) -> &str;
    
    /// Set configuration for the adapter (optional)
    async fn configure(&self, _config: serde_json::Value) -> Result<()> {
        Ok(()) // Default implementation does nothing
    }
}

/// State for managing service connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Main service that manages adapters and the event bus
pub struct StreamService {
    event_bus: Arc<EventBus>,
    adapters: Arc<RwLock<HashMap<String, Arc<Box<dyn ServiceAdapter>>>>>,
    status: Arc<RwLock<HashMap<String, ServiceStatus>>>,
    ws_server_handle: Option<tokio::task::JoinHandle<()>>,
    shutdown_sender: Option<mpsc::Sender<()>>,
}

impl StreamService {
    pub fn new() -> Self {
        let event_bus = Arc::new(EventBus::new(EVENT_BUS_CAPACITY));
        Self {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            ws_server_handle: None,
            shutdown_sender: None,
        }
    }

    /// Register a new adapter with the service
    pub async fn register_adapter<A>(&self, adapter: A) where A: ServiceAdapter + 'static {
        let name = adapter.get_name().to_string();
        let adapter_box = Arc::new(Box::new(adapter) as Box<dyn ServiceAdapter>);
        
        self.adapters.write().await.insert(name.clone(), adapter_box);
        self.status.write().await.insert(name, ServiceStatus::Disconnected);
    }

    /// Connect all registered adapters
    pub async fn connect_all_adapters(&self) -> Result<()> {
        let adapter_names: Vec<String> = {
            self.adapters.read().await.keys().cloned().collect()
        };

        for name in adapter_names {
            self.connect_adapter(&name).await?;
        }

        Ok(())
    }

    /// Connect a specific adapter by name
    pub async fn connect_adapter(&self, name: &str) -> Result<()> {
        let adapter = {
            match self.adapters.read().await.get(name) {
                Some(adapter) => adapter.clone(),
                None => return Err(anyhow!("Adapter '{}' not found", name)),
            }
        };

        {
            let mut status = self.status.write().await;
            status.insert(name.to_string(), ServiceStatus::Connecting);
        }

        // Spawn a task for connection with retry logic
        let adapter_clone = adapter.clone();
        let name_clone = name.to_string();
        let status_clone = Arc::clone(&self.status);

        tokio::spawn(async move {
            loop {
                match adapter_clone.connect().await {
                    Ok(()) => {
                        status_clone.write().await.insert(name_clone.clone(), ServiceStatus::Connected);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to connect adapter '{}': {}. Retrying in {}ms...", 
                            name_clone, e, RECONNECT_DELAY_MS);
                        
                        status_clone.write().await.insert(name_clone.clone(), ServiceStatus::Error);
                        sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Disconnect a specific adapter by name
    pub async fn disconnect_adapter(&self, name: &str) -> Result<()> {
        let adapter = {
            match self.adapters.read().await.get(name) {
                Some(adapter) => adapter.clone(),
                None => return Err(anyhow!("Adapter '{}' not found", name)),
            }
        };

        adapter.disconnect().await?;
        self.status.write().await.insert(name.to_string(), ServiceStatus::Disconnected);
        Ok(())
    }

    /// Get the status of a specific adapter
    pub async fn get_adapter_status(&self, name: &str) -> Result<ServiceStatus> {
        match self.status.read().await.get(name) {
            Some(status) => Ok(*status),
            None => Err(anyhow!("Adapter '{}' not found", name)),
        }
    }

    /// Get all adapter statuses
    pub async fn get_all_statuses(&self) -> HashMap<String, ServiceStatus> {
        self.status.read().await.clone()
    }

    /// Get a reference to the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Start the WebSocket server for external clients
    pub async fn start_websocket_server(&mut self) -> Result<()> {
        if self.ws_server_handle.is_some() {
            return Err(anyhow!("WebSocket server already running"));
        }

        println!("Starting WebSocket server...");
        let event_bus = self.event_bus.clone();
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel::<()>(1);
        
        // Store sender for later shutdown
        self.shutdown_sender = Some(shutdown_sender);

        // Start WebSocket server in a separate task
        let handle = tokio::spawn(async move {
            let socket_addr = format!("127.0.0.1:{}", WS_PORT).parse::<std::net::SocketAddr>().unwrap();
            let listener = match TcpListener::bind(socket_addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    eprintln!("Failed to bind WebSocket server: {}", e);
                    return;
                }
            };

            println!("WebSocket server listening on: {}", socket_addr);

            // Handle incoming connections
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_receiver.recv() => {
                        println!("WebSocket server shutting down");
                        break;
                    }
                    
                    // Accept new connections
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                println!("New WebSocket connection from: {}", addr);
                                
                                // Handle each connection in a separate task
                                let event_bus_clone = event_bus.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_websocket_client(stream, event_bus_clone).await {
                                        eprintln!("Error in WebSocket connection: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Failed to accept WebSocket connection: {}", e);
                            }
                        }
                    }
                }
            }
        });

        self.ws_server_handle = Some(handle);
        Ok(())
    }

    /// Stop the WebSocket server
    pub async fn stop_websocket_server(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(()).await;
            
            if let Some(handle) = self.ws_server_handle.take() {
                handle.await?;
            }
            
            Ok(())
        } else {
            Err(anyhow!("WebSocket server not running"))
        }
    }

    /// Start REST API for service control
    pub async fn start_http_api(&self) -> Result<()> {
        // Create shared state for the API handlers
        let state = ApiState {
            event_bus: self.event_bus.clone(),
            status: Arc::clone(&self.status),
            adapters: Arc::clone(&self.adapters),
        };

        // Build our routes using Axum
        let app = Router::new()
            .route("/stats", get(get_stats_handler))
            .route("/events", get(stream_events_handler))
            .route("/status", get(get_status_handler))
            .route("/adapters/:name/connect", post(connect_adapter_handler))
            .route("/adapters/:name/disconnect", post(disconnect_adapter_handler))
            .with_state(state);

        // Start the server in a background task
        let http_port = WS_PORT + 1; // Use next port for HTTP
        tokio::spawn(async move {
            println!("Starting HTTP API server on port {}", http_port);
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", http_port))
                .await
                .unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        Ok(())
    }
}

// API state shared across all handlers
#[derive(Clone)]
struct ApiState {
    event_bus: Arc<EventBus>,
    status: Arc<RwLock<HashMap<String, ServiceStatus>>>,
    adapters: Arc<RwLock<HashMap<String, Arc<Box<dyn ServiceAdapter>>>>>,
}

// Wrapper type for anyhow::Error to implement IntoResponse
struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        ).into_response()
    }
}

// Handler functions for Axum
async fn get_stats_handler(
    State(state): State<ApiState>,
) -> Json<EventBusStats> {
    let stats = state.event_bus.get_stats().await;
    Json(stats)
}

async fn stream_events_handler(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // This would be a streaming response with Server-Sent Events
    // For simplicity, we'll just return the current stats as JSON
    let stats = state.event_bus.get_stats().await;
    Json(stats)
}

async fn get_status_handler(
    State(state): State<ApiState>,
) -> Json<HashMap<String, ServiceStatus>> {
    let status_map = state.status.read().await.clone();
    Json(status_map)
}

async fn connect_adapter_handler(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<String>, StatusCode> {
    // Update the status to connecting
    state.status.write().await.insert(name.clone(), ServiceStatus::Connecting);
    
    match state.adapters.read().await.get(&name) {
        Some(adapter) => {
            let adapter_clone = adapter.clone();
            let name_clone = name.clone();
            tokio::spawn(async move {
                if let Err(e) = adapter_clone.connect().await {
                    eprintln!("Failed to connect adapter '{}': {}", name_clone, e);
                }
            });
            Ok(Json("Connection initiated".to_string()))
        }
        None => {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

async fn disconnect_adapter_handler(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<String>, StatusCode> {
    match state.adapters.read().await.get(&name) {
        Some(adapter) => {
            let adapter_clone = adapter.clone();
            let name_clone = name.clone();
            tokio::spawn(async move {
                if let Err(e) = adapter_clone.disconnect().await {
                    eprintln!("Failed to disconnect adapter '{}': {}", name_clone, e);
                }
            });
            state.status.write().await.insert(name, ServiceStatus::Disconnected);
            Ok(Json("Disconnection initiated".to_string()))
        }
        None => {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Handler for WebSocket client connections
async fn handle_websocket_client(
    stream: TcpStream,
    event_bus: Arc<EventBus>,
) -> Result<()> {
    // Upgrade TCP connection to WebSocket
    let ws_stream = accept_async(stream).await?;
    
    // Create a receiver from the event bus
    let mut receiver = event_bus.subscribe();
    
    // Split the WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Handle incoming WebSocket messages
    loop {
        tokio::select! {
            // Handle events from the event bus
            result = receiver.recv() => {
                match result {
                    Ok(event) => {
                        // Serialize event to JSON and send to client
                        match serde_json::to_string(&event) {
                            Ok(json) => {
                                if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
                                    eprintln!("Error sending WebSocket message: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error serializing event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving from event bus: {}", e);
                        break;
                    }
                }
            }
            
            // Handle incoming WebSocket messages
            result = ws_receiver.next() => {
                match result {
                    Some(Ok(msg)) => {
                        // Handle client messages - could implement filtering/commands here
                        if msg.is_close() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                    None => break, // Client disconnected
                }
            }
        }
    }

    Ok(())
}
