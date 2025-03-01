use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

// Export modules
pub mod adapters;
pub mod auth;
pub mod events;

use adapters::{AdapterManager, AdapterSettings, AdapterStatus};
use auth::{AuthService, SecureTokenStore};
use events::{EventBus, EVENT_BUS_CAPACITY, EVENT_BUFFER_SIZE, StreamEvent};

// Default WebSocket server port
pub const DEFAULT_WEBSOCKET_PORT: u16 = 9000;

/// Configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// Adapter settings
    pub adapters: HashMap<String, AdapterSettings>,
}

/// Configuration for the WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Port to bind the WebSocket server to
    pub port: u16,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_WEBSOCKET_PORT,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig::default(),
            adapters: HashMap::new(),
        }
    }
}

/// Information about the WebSocket server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketInfo {
    /// Port the server is listening on
    pub port: u16,
    /// Whether the server is running
    pub running: bool,
    /// URI for connecting to the server
    pub uri: String,
}

/// Main application state
pub struct AppState {
    /// Event bus for distributing events
    pub event_bus: Arc<EventBus>,
    /// Adapter manager for handling service adapters
    pub adapter_manager: Arc<AdapterManager>,
    /// Authentication service
    pub auth_service: Arc<AuthService>,
    /// WebSocket server handle
    websocket_server: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// WebSocket server shutdown channel
    websocket_shutdown: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    /// WebSocket server configuration
    websocket_config: Arc<RwLock<WebSocketConfig>>,
}

impl AppState {
    /// Create a new application state
    pub fn new(app_handle: Option<Arc<tauri::AppHandle>>) -> Self {
        // Create the event bus
        let event_bus = Arc::new(EventBus::new(EVENT_BUS_CAPACITY, EVENT_BUFFER_SIZE));
        
        // Create secure token store
        let secure_token_store = match app_handle {
            Some(handle) => Arc::new(SecureTokenStore::new("zelan.secure.json").with_app_handle(handle)),
            None => Arc::new(SecureTokenStore::new("zelan.secure.json")),
        };
        
        // Create auth service
        let auth_service = Arc::new(AuthService::new(secure_token_store));
        
        // Create adapter manager
        let adapter_manager = Arc::new(AdapterManager::new(event_bus.clone()));
        
        Self {
            event_bus,
            adapter_manager,
            auth_service,
            websocket_server: Arc::new(RwLock::new(None)),
            websocket_shutdown: Arc::new(RwLock::new(None)),
            websocket_config: Arc::new(RwLock::new(WebSocketConfig::default())),
        }
    }
    
    /// Initialize the application
    pub async fn initialize(&self, app_handle: Option<Arc<tauri::AppHandle>>) -> Result<()> {
        info!("Initializing application");
        
        // Initialize the auth service
        self.auth_service.initialize(app_handle).await?;
        
        Ok(())
    }
    
    /// Start the WebSocket server
    pub async fn start_websocket_server(&self) -> Result<()> {
        // Check if server is already running
        if self.websocket_server.read().await.is_some() {
            return Err(anyhow!("WebSocket server already running"));
        }
        
        // Get the port from config
        let port = self.websocket_config.read().await.port;
        info!(port = port, "Starting WebSocket server");
        
        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        
        // Store the shutdown sender
        *self.websocket_shutdown.write().await = Some(shutdown_tx);
        
        // Clone event bus for the server task
        let event_bus = self.event_bus.clone();
        
        // Start the server in a background task
        let handle = tokio::spawn(async move {
            // Bind to the port
            let addr = format!("127.0.0.1:{}", port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    error!(error = %e, "Failed to bind WebSocket server");
                    return;
                }
            };
            
            info!("WebSocket server listening on {}", addr);
            
            loop {
                tokio::select! {
                    // Accept new connections
                    Ok((stream, peer_addr)) = listener.accept() => {
                        info!("New WebSocket connection from {}", peer_addr);
                        
                        // Handle the connection in a separate task
                        let event_bus_clone = event_bus.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_websocket_client(stream, event_bus_clone).await {
                                error!(error = %e, "Error in WebSocket connection");
                            }
                        });
                    }
                    
                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!("WebSocket server shutting down");
                        break;
                    }
                }
            }
        });
        
        // Store the server handle
        *self.websocket_server.write().await = Some(handle);
        
        Ok(())
    }
    
    /// Stop the WebSocket server
    pub async fn stop_websocket_server(&self) -> Result<()> {
        // Check if server is running
        if self.websocket_server.read().await.is_none() {
            return Err(anyhow!("WebSocket server not running"));
        }
        
        info!("Stopping WebSocket server");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = self.websocket_shutdown.write().await.take() {
            let _ = shutdown_tx.send(()).await;
        }
        
        // Wait for server task to complete
        if let Some(handle) = self.websocket_server.write().await.take() {
            let _ = handle.await;
        }
        
        Ok(())
    }
    
    /// Get information about the WebSocket server
    pub async fn get_websocket_info(&self) -> WebSocketInfo {
        let config = self.websocket_config.read().await;
        let running = self.websocket_server.read().await.is_some();
        
        WebSocketInfo {
            port: config.port,
            running,
            uri: format!("ws://127.0.0.1:{}", config.port),
        }
    }
    
    /// Set the WebSocket server port
    pub async fn set_websocket_port(&self, port: u16) -> Result<()> {
        // Check if server is running
        if self.websocket_server.read().await.is_some() {
            return Err(anyhow!("Cannot change port while WebSocket server is running"));
        }
        
        info!(port = port, "Setting WebSocket server port");
        
        // Update the config
        self.websocket_config.write().await.port = port;
        
        Ok(())
    }
    
    /// Add a Twitch adapter
    pub async fn add_twitch_adapter(&self) -> Result<()> {
        // Create a Twitch adapter
        let twitch_adapter = adapters::twitch::TwitchAdapter::new(
            self.event_bus.clone(),
            self.auth_service.clone(),
        );
        
        // Register the adapter
        self.adapter_manager.register_adapter(
            twitch_adapter,
            Some(AdapterSettings {
                enabled: true,
                config: serde_json::json!({}),
                display_name: "Twitch".to_string(),
                description: "Adapter for Twitch streaming platform".to_string(),
            }),
        ).await;
        
        Ok(())
    }
}

/// Handler for WebSocket client connections
async fn handle_websocket_client(stream: TcpStream, event_bus: Arc<EventBus>) -> Result<()> {
    // Get peer address for logging
    let peer_addr = stream.peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
        
    // Upgrade to WebSocket
    let ws_stream = accept_async(stream).await
        .map_err(|e| anyhow!("WebSocket handshake failed: {}", e))?;
        
    debug!("WebSocket connection established with {}", peer_addr);
    
    // Subscribe to the event bus
    let mut subscriber = event_bus.subscribe();
    
    // Get history from the subscriber
    let history = subscriber.replay_buffer().await;
    
    // Split the WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Send initial history events, if any
    for event in history {
        match serde_json::to_string(&event) {
            Ok(json) => {
                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                    error!(error = %e, "Failed to send history event to WebSocket client");
                    break;
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to serialize event for WebSocket client");
            }
        }
    }
    
    // Send welcome message
    if let Err(e) = ws_sender.send(Message::Text(serde_json::to_string(&serde_json::json!({
        "source": "zelan",
        "event_type": "system.connected",
        "payload": {
            "message": "Connected to Zelan event stream",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }
    })).unwrap())).await {
        error!(error = %e, "Failed to send welcome message to WebSocket client");
        return Err(anyhow!("Failed to send welcome message"));
    }
    
    // Metrics
    let connection_start = std::time::Instant::now();
    let mut messages_sent = 0;
    let mut last_activity = std::time::Instant::now();
    
    // Main event loop
    loop {
        // Check if client has been inactive for too long (5 minutes)
        if last_activity.elapsed() > Duration::from_secs(300) {
            info!("WebSocket client {} timed out due to inactivity", peer_addr);
            break;
        }
        
        tokio::select! {
            // Handle events from the event bus
            event = subscriber.recv() => {
                match event {
                    Ok(event) => {
                        match serde_json::to_string(&event) {
                            Ok(json) => {
                                if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                    error!(error = %e, "Failed to send event to WebSocket client");
                                    break;
                                }
                                messages_sent += 1;
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to serialize event for WebSocket client");
                            }
                        }
                    }
                    Err(e) => {
                        // If we got a lagged error, reconnect to the event bus
                        if e.to_string().contains("lagged") {
                            warn!("WebSocket client {} lagged behind, reconnecting to event bus", peer_addr);
                            subscriber = event_bus.subscribe();
                            continue;
                        } else {
                            error!(error = %e, "Error receiving from event bus");
                            break;
                        }
                    }
                }
            }
            
            // Handle messages from the client
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        // Update last activity
                        last_activity = std::time::Instant::now();
                        
                        match msg {
                            Message::Text(text) => {
                                debug!("Received text message from {}: {}", peer_addr, text);
                                
                                // Handle ping/pong
                                if text == "ping" {
                                    if let Err(e) = ws_sender.send(Message::Text("pong".into())).await {
                                        error!(error = %e, "Failed to send pong response");
                                        break;
                                    }
                                }
                            }
                            Message::Ping(data) => {
                                // Respond to ping with pong
                                if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                    error!(error = %e, "Failed to send pong response");
                                    break;
                                }
                            }
                            Message::Pong(_) => {
                                // Client responded to our ping
                                debug!("Received pong from {}", peer_addr);
                            }
                            Message::Close(_) => {
                                info!("WebSocket client {} requested close", peer_addr);
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "WebSocket error from {}", peer_addr);
                        break;
                    }
                    None => {
                        info!("WebSocket client {} disconnected", peer_addr);
                        break;
                    }
                }
            }
            
            // Periodic ping to check connection
            _ = sleep(Duration::from_secs(60)) => {
                debug!("Sending ping to {}", peer_addr);
                if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
                    error!(error = %e, "Failed to send ping to {}", peer_addr);
                    break;
                }
            }
        }
    }
    
    // Log disconnection
    let duration = connection_start.elapsed();
    info!(
        client = %peer_addr,
        duration_secs = duration.as_secs(),
        messages = messages_sent,
        "WebSocket client disconnected"
    );
    
    Ok(())
}