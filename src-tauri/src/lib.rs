use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, span, warn, Instrument, Level};

// Re-export Adapters
pub mod adapters;
pub mod error;
pub mod plugin;

pub use error::{ErrorCode, ErrorSeverity, ZelanError, ZelanResult};

// Constants for event bus configuration
const EVENT_BUS_CAPACITY: usize = 1000;
const RECONNECT_DELAY_MS: u64 = 5000;
const DEFAULT_WS_PORT: u16 = 9000;

/// Configuration for the entire application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// Adapter settings keyed by adapter name
    pub adapters: HashMap<String, AdapterSettings>,
}

/// Configuration for the WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Port to bind the WebSocket server to
    pub port: u16,
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    events_published: u64,
    events_dropped: u64,
    source_counts: HashMap<String, u64>,
    type_counts: HashMap<String, u64>,
}

impl Default for EventBusStats {
    fn default() -> Self {
        Self {
            events_published: 0,
            events_dropped: 0,
            source_counts: HashMap::new(),
            type_counts: HashMap::new(),
        }
    }
}

impl EventBus {
    #[instrument(level = "debug")]
    pub fn new(capacity: usize) -> Self {
        info!(capacity, "Creating new event bus");
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            buffer_size: capacity,
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Get a subscriber to receive events
    #[instrument(skip(self), level = "debug")]
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        debug!("New subscriber registered to event bus");
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers with optimized statistics handling
    #[instrument(skip(self, event), fields(source = %event.source(), event_type = %event.event_type()), level = "debug")]
    pub async fn publish(&self, event: StreamEvent) -> ZelanResult<usize> {
        // Cache event details before acquiring lock
        let source = event.source.clone();
        let event_type = event.event_type.clone();

        debug!("Publishing event to bus");

        // Attempt to send the event first (most common operation)
        match self.sender.send(event) {
            Ok(receivers) => {
                // Only update stats after successful send, using a separate task
                let stats = Arc::clone(&self.stats);
                tauri::async_runtime::spawn(async move {
                    let mut stats_guard = stats.write().await;
                    stats_guard.events_published += 1;
                    *stats_guard.source_counts.entry(source).or_insert(0) += 1;
                    *stats_guard.type_counts.entry(event_type).or_insert(0) += 1;
                });

                debug!(receivers, "Event published successfully");
                Ok(receivers)
            }
            Err(err) => {
                // If error indicates no subscribers, just record the statistic but don't treat as error
                if err.to_string().contains("channel closed")
                    || err.to_string().contains("no receivers")
                {
                    // Update dropped count in a separate task
                    let stats = Arc::clone(&self.stats);
                    tauri::async_runtime::spawn(async move {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_dropped += 1;
                    });

                    debug!(
                        source = %source,
                        event_type = %event_type,
                        "No receivers for event, message dropped"
                    );
                    Ok(0) // Return 0 receivers instead of an error
                } else {
                    // Update dropped count for other errors
                    let stats = Arc::clone(&self.stats);
                    tauri::async_runtime::spawn(async move {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_dropped += 1;
                    });

                    error!(error = %err, "Failed to publish event");
                    Err(error::event_bus_publish_failed(err))
                }
            }
        }
    }

    /// Get current event bus statistics
    #[instrument(skip(self), level = "debug")]
    pub async fn get_stats(&self) -> EventBusStats {
        let stats = self.stats.read().await.clone();
        debug!(
            events_published = stats.events_published,
            events_dropped = stats.events_dropped,
            "Retrieved event bus statistics"
        );
        stats
    }

    /// Reset all statistics counters
    #[instrument(skip(self), level = "debug")]
    pub async fn reset_stats(&self) {
        info!("Resetting event bus statistics");
        *self.stats.write().await = EventBusStats::default();
        debug!("Event bus statistics reset to defaults");
    }

    /// Get the configured capacity of the event bus
    #[instrument(skip(self), level = "debug")]
    pub fn capacity(&self) -> usize {
        debug!(capacity = self.buffer_size, "Retrieved event bus capacity");
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
    Disabled,
}

/// Settings for a service adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterSettings {
    /// Whether the adapter is enabled
    pub enabled: bool,
    /// Adapter-specific configuration
    #[serde(default)]
    pub config: serde_json::Value,
    /// Display name for the adapter
    pub display_name: String,
    /// Description of the adapter's functionality
    pub description: String,
}

/// Main service that manages adapters and the event bus
pub struct StreamService {
    event_bus: Arc<EventBus>,
    adapters: Arc<RwLock<HashMap<String, Arc<Box<dyn ServiceAdapter>>>>>,
    status: Arc<RwLock<HashMap<String, ServiceStatus>>>,
    adapter_settings: Arc<RwLock<HashMap<String, AdapterSettings>>>,
    ws_server_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    shutdown_sender: Option<mpsc::Sender<()>>,
    ws_config: WebSocketConfig,
}

// Implement Clone for StreamService so it can be used in async contexts
impl Clone for StreamService {
    fn clone(&self) -> Self {
        Self {
            event_bus: self.event_bus.clone(),
            adapters: self.adapters.clone(),
            status: self.status.clone(),
            adapter_settings: self.adapter_settings.clone(),
            ws_server_handle: None, // We don't clone the server handle
            shutdown_sender: None,  // We don't clone the shutdown sender
            ws_config: self.ws_config.clone(),
        }
    }
}

impl StreamService {
    #[instrument(level = "info")]
    pub fn new() -> Self {
        info!("Creating new StreamService with default configuration");
        let event_bus = Arc::new(EventBus::new(EVENT_BUS_CAPACITY));
        Self {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            adapter_settings: Arc::new(RwLock::new(HashMap::new())),
            ws_server_handle: None,
            shutdown_sender: None,
            ws_config: WebSocketConfig {
                port: DEFAULT_WS_PORT,
            },
        }
    }

    /// Create a new StreamService with custom WebSocket configuration
    #[instrument(fields(port = ws_config.port), level = "info")]
    pub fn with_config(ws_config: WebSocketConfig) -> Self {
        info!("Creating new StreamService with custom WebSocket configuration");
        let event_bus = Arc::new(EventBus::new(EVENT_BUS_CAPACITY));
        Self {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            adapter_settings: Arc::new(RwLock::new(HashMap::new())),
            ws_server_handle: None,
            shutdown_sender: None,
            ws_config,
        }
    }

    /// Load configuration from app storage
    #[instrument(skip(config), level = "info")]
    pub fn from_config(config: Config) -> Self {
        info!(
            adapter_count = config.adapters.len(),
            ws_port = config.websocket.port,
            "Creating StreamService from stored configuration"
        );

        let event_bus = Arc::new(EventBus::new(EVENT_BUS_CAPACITY));
        let adapter_settings = Arc::new(RwLock::new(config.adapters));

        Self {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            adapter_settings,
            ws_server_handle: None,
            shutdown_sender: None,
            ws_config: config.websocket,
        }
    }

    /// Export the current configuration
    #[instrument(skip(self), level = "debug")]
    pub async fn export_config(&self) -> Config {
        debug!("Exporting current StreamService configuration");
        let adapter_settings = self.adapter_settings.read().await.clone();
        let config = Config {
            websocket: self.ws_config.clone(),
            adapters: adapter_settings,
        };

        debug!(
            ws_port = config.websocket.port,
            adapter_count = config.adapters.len(),
            "Configuration exported"
        );

        config
    }

    /// Get the current WebSocket configuration
    #[instrument(skip(self), level = "debug")]
    pub fn ws_config(&self) -> &WebSocketConfig {
        debug!(
            port = self.ws_config.port,
            "Retrieved WebSocket configuration"
        );
        &self.ws_config
    }

    /// Update the WebSocket configuration
    #[instrument(skip(self), fields(old_port = self.ws_config.port, new_port = config.port), level = "info")]
    pub fn set_ws_config(&mut self, config: WebSocketConfig) {
        info!("Updating WebSocket configuration");
        self.ws_config = config;
        debug!("WebSocket configuration updated");
    }

    /// Register a new adapter with the service
    pub async fn register_adapter<A>(&self, adapter: A, settings: Option<AdapterSettings>)
    where
        A: ServiceAdapter + 'static,
    {
        let name = adapter.get_name().to_string();
        let adapter_box = Arc::new(Box::new(adapter) as Box<dyn ServiceAdapter>);

        // Store the adapter
        self.adapters
            .write()
            .await
            .insert(name.clone(), adapter_box);

        // Create default settings if none provided
        let adapter_settings = settings.unwrap_or_else(|| AdapterSettings {
            enabled: true,
            config: serde_json::Value::Null,
            display_name: name.clone(),
            description: format!("{} adapter", name),
        });

        // Store the settings
        self.adapter_settings
            .write()
            .await
            .insert(name.clone(), adapter_settings);

        // Set initial status based on enabled setting
        let initial_status = if self
            .adapter_settings
            .read()
            .await
            .get(&name)
            .unwrap()
            .enabled
        {
            ServiceStatus::Disconnected
        } else {
            ServiceStatus::Disabled
        };

        self.status.write().await.insert(name, initial_status);
    }

    /// Get adapter settings
    pub async fn get_adapter_settings(&self, name: &str) -> Result<AdapterSettings> {
        match self.adapter_settings.read().await.get(name) {
            Some(settings) => Ok(settings.clone()),
            None => Err(anyhow!("Adapter '{}' not found", name)),
        }
    }

    /// Get all adapter settings
    pub async fn get_all_adapter_settings(&self) -> HashMap<String, AdapterSettings> {
        self.adapter_settings.read().await.clone()
    }

    /// Update adapter settings
    pub async fn update_adapter_settings(
        &self,
        name: &str,
        settings: AdapterSettings,
    ) -> ZelanResult<()> {
        // Check if adapter exists
        if !self.adapters.read().await.contains_key(name) {
            return Err(error::adapter_not_found(name));
        }

        // Get old settings to check if enabled status changed
        let old_enabled = match self.adapter_settings.read().await.get(name) {
            Some(old_settings) => old_settings.enabled,
            None => true, // Default to enabled if no previous settings
        };

        // Update settings
        self.adapter_settings
            .write()
            .await
            .insert(name.to_string(), settings.clone());

        // Handle enable/disable if the status changed
        if old_enabled != settings.enabled {
            if settings.enabled {
                // Changed from disabled to enabled - set to disconnected
                self.status
                    .write()
                    .await
                    .insert(name.to_string(), ServiceStatus::Disconnected);

                // Automatically connect the adapter when enabled
                let adapter_clone = self.adapters.read().await.get(name).cloned();
                if let Some(adapter) = adapter_clone {
                    println!("Auto-connecting newly enabled adapter: {}", name);
                    let name_clone = name.to_string(); // Clone the name to satisfy lifetime requirements
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = adapter.connect().await {
                            eprintln!("Failed to auto-connect adapter '{}': {}", name_clone, e);
                        }
                    });
                }
            } else {
                // Changed from enabled to disabled - disconnect if connected
                let current_status = self.status.read().await.get(name).cloned();

                if let Some(status) = current_status {
                    if status == ServiceStatus::Connected || status == ServiceStatus::Connecting {
                        match self.adapters.read().await.get(name) {
                            Some(adapter) => {
                                let _ = adapter.disconnect().await;
                            }
                            None => {}
                        }
                    }

                    // Set status to disabled
                    self.status
                        .write()
                        .await
                        .insert(name.to_string(), ServiceStatus::Disabled);
                }
            }
        }

        // If settings contains configuration, apply it to the adapter
        if !settings.config.is_null() {
            if let Some(adapter) = self.adapters.read().await.get(name) {
                if let Err(e) = adapter.configure(settings.config.clone()).await {
                    return Err(ZelanError {
                        code: ErrorCode::ConfigInvalid,
                        message: format!("Failed to configure adapter '{}'", name),
                        context: Some(e.to_string()),
                        severity: ErrorSeverity::Warning,
                    });
                }
            }
        }

        Ok(())
    }

    /// Connect all registered adapters that are enabled
    pub async fn connect_all_adapters(&self) -> Result<()> {
        let adapter_names: Vec<String> = { self.adapters.read().await.keys().cloned().collect() };

        for name in adapter_names {
            // Skip disabled adapters
            let is_enabled = match self.adapter_settings.read().await.get(&name) {
                Some(settings) => settings.enabled,
                None => true, // Default to enabled if no settings
            };

            if is_enabled {
                // Don't fail on individual adapter connection failures
                if let Err(e) = self.connect_adapter(&name).await {
                    eprintln!("Failed to connect adapter '{}': {}", name, e);
                }
            } else {
                println!("Skipping disabled adapter: {}", name);
            }
        }

        Ok(())
    }

    /// Connect a specific adapter by name
    pub async fn connect_adapter(&self, name: &str) -> ZelanResult<()> {
        // Check if adapter exists
        let adapter = {
            match self.adapters.read().await.get(name) {
                Some(adapter) => adapter.clone(),
                None => return Err(error::adapter_not_found(name)),
            }
        };

        // Check if adapter is enabled
        let is_enabled = match self.adapter_settings.read().await.get(name) {
            Some(settings) => settings.enabled,
            None => true, // Default to enabled if no settings
        };

        if !is_enabled {
            return Err(ZelanError {
                code: ErrorCode::AdapterDisabled,
                message: format!("Adapter '{}' is disabled", name),
                context: Some("Enable the adapter in settings before connecting".to_string()),
                severity: ErrorSeverity::Warning,
            });
        }

        {
            let mut status = self.status.write().await;
            status.insert(name.to_string(), ServiceStatus::Connecting);
        }

        // Spawn a task for connection with retry logic
        let adapter_clone = adapter.clone();
        let name_clone = name.to_string();
        let status_clone = Arc::clone(&self.status);

        // Track connection attempts for exponential backoff
        let max_retries = 5; // Maximum number of retries
        let mut retry_count = 0;

        tauri::async_runtime::spawn(async move {
            loop {
                match adapter_clone.connect().await {
                    Ok(()) => {
                        status_clone
                            .write()
                            .await
                            .insert(name_clone.clone(), ServiceStatus::Connected);
                        println!("Adapter '{}' connected successfully", name_clone);
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        let err = error::adapter_connection_failed(&name_clone, &e);

                        // Calculate backoff time with exponential increase
                        let backoff_ms = if retry_count >= max_retries {
                            RECONNECT_DELAY_MS * 5 // Cap at 5x the base delay
                        } else {
                            RECONNECT_DELAY_MS * (1 << retry_count.min(6)) // Exponential backoff with a reasonable cap
                        };

                        eprintln!(
                            "{}. Retry {}/{} in {}ms...",
                            err, retry_count, max_retries, backoff_ms
                        );

                        status_clone
                            .write()
                            .await
                            .insert(name_clone.clone(), ServiceStatus::Error);

                        // If we've reached max retries, wait longer between attempts
                        sleep(Duration::from_millis(backoff_ms)).await;

                        // Consider breaking if too many retries, but continue for now
                        if retry_count > 100 {
                            eprintln!("Too many retries for adapter '{}', giving up", name_clone);
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Disconnect a specific adapter by name
    pub async fn disconnect_adapter(&self, name: &str) -> ZelanResult<()> {
        let adapter = {
            match self.adapters.read().await.get(name) {
                Some(adapter) => adapter.clone(),
                None => return Err(error::adapter_not_found(name)),
            }
        };

        // Set status to disconnecting first
        self.status
            .write()
            .await
            .insert(name.to_string(), ServiceStatus::Disconnected);

        // Then attempt the disconnect
        match adapter.disconnect().await {
            Ok(()) => Ok(()),
            Err(e) => Err(ZelanError {
                code: ErrorCode::AdapterDisconnectFailed,
                message: format!("Failed to disconnect adapter '{}'", name),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Warning,
            }),
        }
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
    #[instrument(skip(self), level = "debug")]
    pub async fn start_websocket_server(&mut self) -> Result<()> {
        if self.ws_server_handle.is_some() {
            warn!("WebSocket server already running");
            return Err(anyhow!("WebSocket server already running"));
        }

        info!("Starting WebSocket server");
        let event_bus = self.event_bus.clone();
        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel::<()>(1);

        // Store sender for later shutdown
        self.shutdown_sender = Some(shutdown_sender);

        // Pass the WebSocket config
        let ws_port = self.ws_config.port;

        // Start WebSocket server in a separate task
        let handle = tauri::async_runtime::spawn(
            async move {
                let socket_addr = format!("127.0.0.1:{}", ws_port)
                    .parse::<std::net::SocketAddr>()
                    .unwrap();
                let listener = match TcpListener::bind(socket_addr).await {
                    Ok(listener) => listener,
                    Err(e) => {
                        error!(error = %e, "Failed to bind WebSocket server");
                        return;
                    }
                };

                // Print a helpful message for connecting to the WebSocket server
                info!(address = %socket_addr, "WebSocket server listening");
                info!(
                    uri = format!("ws://127.0.0.1:{}", ws_port),
                    "WebSocket server available at"
                );
                debug!("Connect with: wscat -c ws://127.0.0.1:{}", ws_port);
                debug!("Connect with: websocat ws://127.0.0.1:{}", ws_port);

                // Handle incoming connections
                loop {
                    tokio::select! {
                        // Check for shutdown signal
                        _ = shutdown_receiver.recv() => {
                            info!("WebSocket server shutting down");
                            break;
                        }

                        // Accept new connections
                        accept_result = listener.accept() => {
                            match accept_result {
                                Ok((stream, addr)) => {
                                    info!(client = %addr, "New WebSocket connection");

                                    // Handle each connection in a separate task
                                    let event_bus_clone = event_bus.clone();
                                    tauri::async_runtime::spawn(
                                        async move {
                                            if let Err(e) = handle_websocket_client(stream, event_bus_clone).await {
                                                error!(error = %e, client = %addr, "Error in WebSocket connection");
                                            }
                                        }
                                        .in_current_span()
                                    );
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to accept WebSocket connection");
                                }
                            }
                        }
                    }
                }
            }
            .instrument(span!(Level::INFO, "ws_server", port = ws_port))
        );

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
            .route(
                "/adapters/:name/disconnect",
                post(disconnect_adapter_handler),
            )
            .with_state(state);

        // Start the server in a background task
        let http_port = self.ws_config.port + 1; // Use next port for HTTP
        tauri::async_runtime::spawn(async move {
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
        )
            .into_response()
    }
}

// Handler functions for Axum
async fn get_stats_handler(State(state): State<ApiState>) -> Json<EventBusStats> {
    let stats = state.event_bus.get_stats().await;
    Json(stats)
}

async fn stream_events_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // This would be a streaming response with Server-Sent Events
    // For simplicity, we'll just return the current stats as JSON
    let stats = state.event_bus.get_stats().await;
    Json(stats)
}

async fn get_status_handler(State(state): State<ApiState>) -> Json<HashMap<String, ServiceStatus>> {
    let status_map = state.status.read().await.clone();
    Json(status_map)
}

async fn connect_adapter_handler(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<String>, StatusCode> {
    // Update the status to connecting
    state
        .status
        .write()
        .await
        .insert(name.clone(), ServiceStatus::Connecting);

    match state.adapters.read().await.get(&name) {
        Some(adapter) => {
            let adapter_clone = adapter.clone();
            let name_clone = name.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = adapter_clone.connect().await {
                    eprintln!("Failed to connect adapter '{}': {}", name_clone, e);
                }
            });
            Ok(Json("Connection initiated".to_string()))
        }
        None => Err(StatusCode::NOT_FOUND),
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
            tauri::async_runtime::spawn(async move {
                if let Err(e) = adapter_clone.disconnect().await {
                    eprintln!("Failed to disconnect adapter '{}': {}", name_clone, e);
                }
            });
            state
                .status
                .write()
                .await
                .insert(name, ServiceStatus::Disconnected);
            Ok(Json("Disconnection initiated".to_string()))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Handler for WebSocket client connections
#[instrument(skip(stream, event_bus), fields(client = ?stream.peer_addr().map_or("unknown".to_string(), |addr| addr.to_string())))]
async fn handle_websocket_client(stream: TcpStream, event_bus: Arc<EventBus>) -> ZelanResult<()> {
    // Get client IP for logging
    let peer_addr = stream
        .peer_addr()
        .map_or("unknown".to_string(), |addr| addr.to_string());

    // Upgrade TCP connection to WebSocket with timeout
    let ws_stream =
        match tokio::time::timeout(std::time::Duration::from_secs(5), accept_async(stream)).await {
            Ok(result) => match result {
                Ok(ws) => ws,
                Err(e) => {
                    error!(error = %e, "WebSocket handshake failed");
                    return Err(error::websocket_accept_failed(e));
                }
            },
            Err(_) => {
                error!("WebSocket handshake timed out");
                return Err(ZelanError {
                    code: ErrorCode::WebSocketAcceptFailed,
                    message: "WebSocket connection timed out during handshake".to_string(),
                    context: Some(format!("Client: {}", peer_addr)),
                    severity: ErrorSeverity::Warning,
                });
            }
        };

    info!(client = %peer_addr, "WebSocket client connected");

    // Create a receiver from the event bus
    let mut receiver = event_bus.subscribe();

    // Split the WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Track connection stats
    let mut messages_sent = 0;
    let mut last_activity = std::time::Instant::now();
    let connection_start = std::time::Instant::now();

    // Keep a cached version of the last serialized events to avoid redundant serialization
    let mut event_cache: Option<(String, String)> = None;

    // Handle incoming WebSocket messages
    loop {
        // Check for client inactivity timeout (5 minutes)
        if last_activity.elapsed() > std::time::Duration::from_secs(300) {
            info!(
                client = %peer_addr,
                duration = ?last_activity.elapsed(),
                "WebSocket client timed out due to inactivity"
            );
            break;
        }

        tokio::select! {
            // Handle events from the event bus
            result = receiver.recv() => {
                match result {
                    Ok(event) => {
                        // Create a key for the event type+source
                        let event_key = format!("{}.{}", event.source(), event.event_type());

                        debug!(
                            client = %peer_addr,
                            event_source = %event.source(),
                            event_type = %event.event_type(),
                            "Forwarding event to client"
                        );

                        // Check if we've already serialized this exact event type+source
                        let json = if let Some((cached_key, cached_json)) = &event_cache {
                            if *cached_key == event_key {
                                cached_json.clone()
                            } else {
                                // Event type changed, serialize new event
                                match serde_json::to_string(&event) {
                                    Ok(json) => {
                                        event_cache = Some((event_key, json.clone()));
                                        json
                                    },
                                    Err(e) => {
                                        error!(
                                            error = %e,
                                            event_source = %event.source(),
                                            event_type = %event.event_type(),
                                            "Error serializing event"
                                        );
                                        continue;
                                    }
                                }
                            }
                        } else {
                            // No cached event, serialize new event
                            match serde_json::to_string(&event) {
                                Ok(json) => {
                                    event_cache = Some((event_key, json.clone()));
                                    json
                                },
                                Err(e) => {
                                    error!(
                                        error = %e,
                                        event_source = %event.source(),
                                        event_type = %event.event_type(),
                                        "Error serializing event"
                                    );
                                    continue;
                                }
                            }
                        };

                        // Send serialized event to client
                        if let Err(e) = ws_sender.send(Message::Text(json.into())).await {
                            error!(
                                error = %e,
                                client = %peer_addr,
                                "Error sending WebSocket message"
                            );
                            break;
                        }

                        // Update stats
                        messages_sent += 1;
                        last_activity = std::time::Instant::now();
                    }
                    Err(e) => {
                        // Don't report lagged errors, just reconnect to the event bus
                        if e.to_string().contains("lagged") {
                            warn!(
                                client = %peer_addr,
                                "WebSocket client lagged behind, reconnecting to event bus"
                            );
                            receiver = event_bus.subscribe();
                            continue;
                        } else {
                            error!(
                                error = %e,
                                client = %peer_addr,
                                "Error receiving from event bus"
                            );
                            break;
                        }
                    }
                }
            }

            // Handle incoming WebSocket messages
            result = ws_receiver.next() => {
                match result {
                    Some(Ok(msg)) => {
                        // Update activity timestamp
                        last_activity = std::time::Instant::now();

                        // Handle client messages
                        match msg {
                            Message::Text(text) => {
                                debug!(client = %peer_addr, message = %text, "Received text message");

                                // Process client commands - could implement filtering/subscriptions here
                                if text == "ping" {
                                    if let Err(e) = ws_sender.send(Message::Text("pong".into())).await {
                                        error!(error = %e, client = %peer_addr, "Error sending pong");
                                        break;
                                    }
                                }
                            },
                            Message::Close(_) => {
                                info!(client = %peer_addr, "WebSocket client requested close");
                                break;
                            },
                            Message::Ping(data) => {
                                debug!(client = %peer_addr, "Received ping");
                                // Automatically respond to pings
                                if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                    error!(error = %e, client = %peer_addr, "Error responding to ping");
                                    break;
                                }
                            },
                            _ => {
                                debug!(client = %peer_addr, message_type = ?msg, "Received other message type");
                            } // Ignore other message types
                        }
                    }
                    Some(Err(e)) => {
                        error!(error = %e, client = %peer_addr, "WebSocket error from client");
                        break;
                    }
                    None => {
                        info!(client = %peer_addr, "WebSocket client disconnected");
                        break;
                    }
                }
            }

            // Add a timeout to check client activity periodically
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                debug!(client = %peer_addr, "Sending ping to check if client is alive");
                // Send ping to check if client is still alive
                if let Err(e) = ws_sender.send(Message::Ping(vec![].into())).await {
                    error!(error = %e, client = %peer_addr, "Error sending ping");
                    break;
                }
            }
        }
    }

    // Log connection statistics
    let duration = connection_start.elapsed();
    info!(
        client = %peer_addr,
        duration = ?duration,
        messages = messages_sent,
        "WebSocket client disconnected"
    );

    Ok(())
}
