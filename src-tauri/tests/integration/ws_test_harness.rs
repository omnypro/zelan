//! WebSocket server test harness
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::json;
use tokio::sync::Mutex;

use zelan_lib::auth::TokenManager;
use zelan_lib::{default_max_connections, default_ping_interval, default_timeout};
use zelan_lib::{Config, EventBus, StreamEvent, StreamService, WebSocketConfig};

use crate::ws_client::WebSocketTestClient;

/// Base test port for WebSocket server
/// Each test should use a unique port to avoid conflicts when tests run in parallel
pub const BASE_TEST_WS_PORT: u16 = 19000;
pub static PORT_COUNTER: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

/// Test environment for WebSocket server testing
pub struct WebSocketTestEnvironment {
    /// Stream service
    pub service: Arc<tokio::sync::Mutex<StreamService>>,
    /// Event bus
    pub event_bus: Arc<EventBus>,
    /// WebSocket clients
    pub clients: Arc<Mutex<HashMap<String, Arc<WebSocketTestClient>>>>,
    /// WS port
    pub ws_port: u16,
}

impl WebSocketTestEnvironment {
    /// Create a new WebSocket test environment
    pub async fn new() -> Result<Self> {
        // We'll get the event bus from the service after creation

        // Create token manager
        let _token_manager = Arc::new(TokenManager::new());

        // Get a unique port for this test
        let port_offset = PORT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let test_port = BASE_TEST_WS_PORT + port_offset;

        // Create WebSocket config
        let ws_config = WebSocketConfig {
            port: test_port,
            max_connections: default_max_connections(),
            timeout_seconds: default_timeout(),
            ping_interval: default_ping_interval(),
        };

        // Create service config
        let config = Config {
            websocket: ws_config.clone(),
            adapters: HashMap::new(),
        };

        // Create stream service using standard constructor
        // The StreamService will create its own EventBus internally
        let service = StreamService::from_config(config.clone());

        let service = Arc::new(tokio::sync::Mutex::new(service));

        // Get the service's EventBus
        let service_guard = service.lock().await;
        let service_event_bus = service_guard.event_bus();
        drop(service_guard);

        let env = Self {
            service,
            event_bus: service_event_bus, // Use the service's EventBus instead of our created one
            clients: Arc::new(Mutex::new(HashMap::new())),
            ws_port: test_port,
        };

        Ok(env)
    }

    /// Start the WebSocket server
    pub async fn start_server(&self) -> Result<()> {
        println!("Starting WebSocket server on port {}", self.ws_port);

        // Get mutable access to the service
        let mut service = self.service.lock().await;
        service.start_websocket_server().await?;

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }

    /// Stop the WebSocket server
    pub async fn stop_server(&self) -> Result<()> {
        println!("Stopping WebSocket server");

        // Get mutable access to the service
        let mut service = self.service.lock().await;
        service.stop_websocket_server().await?;

        Ok(())
    }

    /// Create a new WebSocket client and connect to the server
    pub async fn create_client(&self, client_id: &str) -> Result<Arc<WebSocketTestClient>> {
        let client = Arc::new(WebSocketTestClient::new(client_id));

        // Connect to the server
        client.connect(self.ws_port).await?;

        // Store the client
        let mut clients = self.clients.lock().await;
        clients.insert(client_id.to_string(), Arc::clone(&client));

        Ok(client)
    }

    /// Get a connected client
    pub async fn get_client(&self, client_id: &str) -> Result<Arc<WebSocketTestClient>> {
        let clients = self.clients.lock().await;

        match clients.get(client_id) {
            Some(client) => Ok(Arc::clone(client)),
            None => Err(anyhow!("Client not found: {}", client_id)),
        }
    }

    /// Publish an event to the event bus
    pub async fn publish_event(
        &self,
        source: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<usize> {
        let event = StreamEvent::new(source, event_type, payload);
        let receivers = self.event_bus.publish(event).await?;
        Ok(receivers)
    }

    /// Publish multiple test events
    pub async fn publish_test_events(&self, count: usize) -> Result<()> {
        for i in 0..count {
            self.publish_event(
                "test",
                "test.event",
                json!({
                    "counter": i,
                    "message": format!("Test event #{}", i),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            )
            .await?;

            // Slightly longer delay between events to ensure reliable delivery
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(())
    }

    /// Wait for a client to receive a specific number of messages
    pub async fn wait_for_client_messages(
        &self,
        client_id: &str,
        count: usize,
        timeout_ms: u64,
    ) -> Result<bool> {
        let client = self.get_client(client_id).await?;
        client.wait_for_messages(count, timeout_ms).await
    }

    /// Disconnect all clients with timeout
    pub async fn disconnect_all_clients(&self) -> Result<()> {
        println!("Disconnecting all clients");
        let clients = self.clients.lock().await;

        for (client_id, client) in clients.iter() {
            // Use timeout in case any client disconnect hangs
            match tokio::time::timeout(Duration::from_secs(2), client.disconnect()).await {
                Ok(result) => {
                    if let Err(e) = result {
                        println!("Error disconnecting client {}: {}", client_id, e);
                    }
                }
                Err(_) => {
                    println!("Timeout disconnecting client {}", client_id);
                }
            }
        }

        println!("All clients disconnected");
        Ok(())
    }

    /// Clean up resources with timeout to prevent hanging
    pub async fn cleanup(&self) -> Result<()> {
        println!("Starting cleanup process");

        // Disconnect all clients with timeout
        match tokio::time::timeout(Duration::from_secs(3), self.disconnect_all_clients()).await {
            Ok(result) => {
                if let Err(e) = result {
                    println!("Error disconnecting clients: {}", e);
                }
            }
            Err(_) => {
                println!("Timeout disconnecting clients");
            }
        }

        // Stop the WebSocket server with timeout
        match tokio::time::timeout(Duration::from_secs(3), self.stop_server()).await {
            Ok(result) => {
                if let Err(e) = result {
                    println!("Error stopping WebSocket server: {}", e);
                }
            }
            Err(_) => {
                println!("Timeout stopping WebSocket server");
            }
        }

        println!("Cleanup process completed");
        Ok(())
    }
}
