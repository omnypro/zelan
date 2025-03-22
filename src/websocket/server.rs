//! WebSocket server implementation
//!
//! This module provides the WebSocket server implementation for
//! streaming events to external clients.

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, warn};

use super::{WebSocketConfig, WebSocketClientPreferences};
use crate::core::EventBus;
use crate::error::{self, ZelanResult};

/// WebSocket server for distributing events to external clients
#[derive(Debug)]
pub struct WebSocketServer {
    /// Configuration for the WebSocket server
    config: WebSocketConfig,
    /// Event bus for receiving events
    event_bus: Arc<EventBus>,
    /// Handle to the running server task
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Sender for shutdown signal
    shutdown_sender: Option<mpsc::Sender<()>>,
    /// Active connections counter
    connections: Arc<AtomicUsize>,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(config: WebSocketConfig, event_bus: Arc<EventBus>) -> Self {
        Self {
            config,
            event_bus,
            server_handle: None,
            shutdown_sender: None,
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Start the WebSocket server
    pub async fn start(&mut self) -> Result<()> {
        if self.server_handle.is_some() {
            warn!("WebSocket server already running");
            return Ok(());
        }

        let (shutdown_sender, mut shutdown_receiver) = mpsc::channel::<()>(1);
        self.shutdown_sender = Some(shutdown_sender);

        let event_bus = self.event_bus.clone();
        let config = self.config.clone();
        let connections = self.connections.clone();

        // Start the server in a background task
        let handle = tokio::spawn(async move {
            let addr = format!("127.0.0.1:{}", config.port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    error!(error = %e, "Failed to bind WebSocket server to address {}", addr);
                    return;
                }
            };

            info!("WebSocket server listening on {}", addr);

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
                                let current_count = connections.load(Ordering::SeqCst);
                                
                                // Check if we've reached the maximum connections
                                if current_count >= config.max_connections {
                                    warn!(
                                        client = %addr,
                                        max_connections = config.max_connections,
                                        current_connections = current_count,
                                        "Maximum WebSocket connections reached, rejecting new connection"
                                    );
                                    // Just drop the connection - it will be closed automatically
                                    continue;
                                }

                                // Increment connection counter
                                connections.fetch_add(1, Ordering::SeqCst);
                                let new_count = connections.load(Ordering::SeqCst);
                                
                                info!(
                                    client = %addr,
                                    connections = new_count,
                                    max_connections = config.max_connections,
                                    "New WebSocket connection"
                                );

                                // Handle each connection in a separate task
                                let event_bus_clone = event_bus.clone();
                                let config_clone = config.clone();
                                let counter_clone = Arc::clone(&connections);
                                
                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_client(stream, event_bus_clone, config_clone).await {
                                        error!(error = %e, client = %addr, "Error in WebSocket connection");
                                    }
                                    
                                    // Decrement connection counter when client disconnects
                                    counter_clone.fetch_sub(1, Ordering::SeqCst);
                                    let remaining = counter_clone.load(Ordering::SeqCst);
                                    debug!(client = %addr, remaining_connections = remaining, "Client disconnected");
                                });
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to accept WebSocket connection");
                            }
                        }
                    }
                }
            }
        });

        self.server_handle = Some(handle);
        Ok(())
    }

    /// Stop the WebSocket server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(()).await;

            if let Some(handle) = self.server_handle.take() {
                handle.await?;
            }

            Ok(())
        } else {
            anyhow::bail!("WebSocket server not running")
        }
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    /// Handle a client connection
    #[instrument(skip(stream, event_bus, config), fields(client = ?stream.peer_addr().map_or("unknown".to_string(), |addr| addr.to_string())))]
    async fn handle_client(
        stream: TcpStream,
        event_bus: Arc<EventBus>,
        config: WebSocketConfig,
    ) -> ZelanResult<()> {
        // Get client IP for logging
        let peer_addr = stream
            .peer_addr()
            .map_or("unknown".to_string(), |addr| addr.to_string());

        // Upgrade TCP connection to WebSocket with timeout
        let ws_stream = match tokio::time::timeout(
            Duration::from_secs(5), 
            accept_async(stream)
        ).await {
            Ok(result) => match result {
                Ok(ws) => ws,
                Err(e) => {
                    error!(error = %e, "WebSocket handshake failed");
                    return Err(error::websocket_accept_failed(e));
                }
            },
            Err(_) => {
                error!("WebSocket handshake timed out");
                return Err(error::ZelanError::new(error::ErrorCode::WebSocketAcceptFailed)
                    .message("WebSocket connection timed out during handshake".to_string())
                    .context(format!("Client: {}", peer_addr))
                    .category(error::ErrorCategory::Network)
                    .severity(error::ErrorSeverity::Warning)
                    .build());
            }
        };

        info!(client = %peer_addr, "WebSocket client connected");

        // Create a receiver from the event bus
        let mut receiver = event_bus.subscribe();

        // Split the WebSocket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Track connection stats
        let mut messages_sent = 0;
        let mut last_activity = Instant::now();
        let connection_start = Instant::now();

        // Client subscription preferences
        let mut preferences = WebSocketClientPreferences::default();

        // Handle incoming WebSocket messages
        loop {
            // Check for client inactivity timeout
            if last_activity.elapsed() > Duration::from_secs(config.timeout_seconds) {
                info!(
                    client = %peer_addr,
                    duration = ?last_activity.elapsed(),
                    timeout_seconds = config.timeout_seconds,
                    "WebSocket client timed out due to inactivity"
                );
                break;
            }

            tokio::select! {
                // Handle events from the event bus
                result = receiver.recv() => {
                    match result {
                        Ok(event) => {
                            // Check if this client should receive this event based on filters
                            if !preferences.should_receive_event(&event) {
                                debug!(
                                    client = %peer_addr,
                                    event_source = %event.source(),
                                    event_type = %event.event_type(),
                                    "Skipping event due to client filters"
                                );
                                continue;
                            }

                            debug!(
                                client = %peer_addr,
                                event_source = %event.source(),
                                event_type = %event.event_type(),
                                "Forwarding event to client"
                            );

                            // Serialize the event
                            let json = match serde_json::to_string(&event) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!(
                                        error = %e,
                                        event_source = %event.source(),
                                        event_type = %event.event_type(),
                                        "Error serializing event"
                                    );
                                    continue;
                                }
                            };

                            // Send serialized event to client
                            if let Err(e) = ws_sender.send(Message::Text(json)).await {
                                error!(
                                    error = %e,
                                    client = %peer_addr,
                                    "Error sending WebSocket message"
                                );
                                break;
                            }

                            // Update stats
                            messages_sent += 1;
                            last_activity = Instant::now();
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
                            last_activity = Instant::now();

                            // Handle client messages
                            match msg {
                                Message::Text(text) => {
                                    debug!(client = %peer_addr, message = %text, "Received text message");

                                    // Process client commands
                                    if text == "ping" {
                                        if let Err(e) = ws_sender.send(Message::Text("pong".into())).await {
                                            error!(error = %e, client = %peer_addr, "Error sending pong");
                                            break;
                                        }
                                    } else {
                                        // Try to parse as JSON
                                        match serde_json::from_str::<serde_json::Value>(&text) {
                                            Ok(json) => {
                                                // Check if it's a command
                                                if let Some(command) = json.get("command").and_then(|c| c.as_str()) {
                                                    let data = json.get("data").unwrap_or(&serde_json::Value::Null);
                                                    
                                                    // Process subscription commands
                                                    if command.starts_with("subscribe.") || command.starts_with("unsubscribe.") {
                                                        match preferences.process_subscription_command(command, data) {
                                                            Ok(response) => {
                                                                // Send success response
                                                                let response_json = serde_json::json!({
                                                                    "success": true,
                                                                    "command": command,
                                                                    "message": response
                                                                });
                                                                if let Err(e) = ws_sender.send(Message::Text(response_json.to_string())).await {
                                                                    error!(error = %e, client = %peer_addr, "Error sending command response");
                                                                    break;
                                                                }
                                                            },
                                                            Err(err) => {
                                                                // Send error response
                                                                let response_json = serde_json::json!({
                                                                    "success": false,
                                                                    "command": command,
                                                                    "error": err
                                                                });
                                                                if let Err(e) = ws_sender.send(Message::Text(response_json.to_string())).await {
                                                                    error!(error = %e, client = %peer_addr, "Error sending command error response");
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                    } else if command == "info" {
                                                        // Send info about available commands
                                                        let info_json = serde_json::json!({
                                                            "success": true,
                                                            "command": "info",
                                                            "data": {
                                                                "commands": [
                                                                    "ping",
                                                                    "info",
                                                                    "subscribe.sources",
                                                                    "subscribe.types",
                                                                    "unsubscribe.all"
                                                                ],
                                                                "active_filters": {
                                                                    "filtering_active": preferences.filtering_active,
                                                                    "sources": preferences.source_filters,
                                                                    "types": preferences.type_filters
                                                                }
                                                            }
                                                        });
                                                        if let Err(e) = ws_sender.send(Message::Text(info_json.to_string())).await {
                                                            error!(error = %e, client = %peer_addr, "Error sending info response");
                                                            break;
                                                        }
                                                    } else {
                                                        // Unknown command
                                                        let response_json = serde_json::json!({
                                                            "success": false,
                                                            "command": command,
                                                            "error": format!("Unknown command: {}", command)
                                                        });
                                                        if let Err(e) = ws_sender.send(Message::Text(response_json.to_string())).await {
                                                            error!(error = %e, client = %peer_addr, "Error sending error response");
                                                            break;
                                                        }
                                                    }
                                                }
                                            },
                                            Err(_) => {
                                                // Not JSON, ignore non-standard messages
                                                debug!(client = %peer_addr, "Ignoring non-JSON message");
                                            }
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
                                    debug!(client = %peer_addr, "Received other message type");
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
                _ = sleep(Duration::from_secs(config.ping_interval)) => {
                    debug!(
                        client = %peer_addr, 
                        ping_interval = config.ping_interval,
                        "Sending ping to check if client is alive"
                    );
                    // Send ping to check if client is still alive
                    if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
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
}