use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::adapters::{ServiceAdapter, ServiceAdapterStatus, TestAdapter, TwitchAdapter};
use crate::core::EventBus;
use crate::error::{Error, Result};
use crate::websocket::WebSocketServer;

/// Core service that manages adapters and coordinates the event bus
pub struct StreamService {
    /// The central event bus used for distributing events between adapters
    event_bus: Arc<EventBus>,
    
    /// Registry of all available adapters
    pub adapters: Arc<RwLock<HashMap<String, Box<dyn ServiceAdapter>>>>,
    
    /// WebSocket server for external clients
    websocket_server: Option<Arc<WebSocketServer>>,
    
    /// Flag indicating if the service is running
    pub running: Arc<RwLock<bool>>,
}

impl StreamService {
    /// Create a new StreamService with initialized components
    pub fn new() -> Self {
        let event_bus = Arc::new(EventBus::new());
        
        StreamService {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            websocket_server: None,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Register a new adapter with the service
    pub async fn register_adapter<T: ServiceAdapter + 'static>(&self, id: &str, adapter: T) -> Result<()> {
        let mut adapters = self.adapters.write().await;
        if adapters.contains_key(id) {
            return Err(Error::invalid_operation(
                format!("Adapter with ID '{}' is already registered", id),
            ));
        }
        
        adapters.insert(id.to_string(), Box::new(adapter));
        Ok(())
    }
    
    /// Get a reference to the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }
    
    /// Start the service, initializing the WebSocket server and connecting registered adapters
    pub async fn start(&mut self, websocket_port: Option<u16>) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::invalid_operation("Service is already running".to_string()));
        }
        
        // Initialize and start WebSocket server
        let websocket_port = websocket_port.unwrap_or(8080);
        let ws_server = WebSocketServer::new(self.event_bus.clone(), websocket_port);
        let ws_server = Arc::new(ws_server);
        
        let ws_server_clone = ws_server.clone();
        tokio::spawn(async move {
            if let Err(e) = ws_server_clone.start().await {
                eprintln!("WebSocket server error: {:?}", e);
            }
        });
        
        self.websocket_server = Some(ws_server);
        
        // Start all registered adapters
        let adapters = self.adapters.read().await;
        for (id, adapter) in adapters.iter() {
            if let Err(e) = adapter.connect().await {
                eprintln!("Failed to connect adapter {}: {:?}", id, e);
            }
        }
        
        *running = true;
        Ok(())
    }
    
    /// Stop the service, disconnecting all adapters and shutting down the WebSocket server
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(Error::invalid_operation("Service is not running".to_string()));
        }
        
        // Disconnect all adapters
        let adapters = self.adapters.read().await;
        for (id, adapter) in adapters.iter() {
            if let Err(e) = adapter.disconnect().await {
                eprintln!("Failed to disconnect adapter {}: {:?}", id, e);
            }
        }
        
        // Stop WebSocket server
        if let Some(ws_server) = &self.websocket_server {
            if let Err(e) = ws_server.stop().await {
                eprintln!("Failed to stop WebSocket server: {:?}", e);
            }
        }
        
        *running = false;
        Ok(())
    }
    
    /// Connect a specific adapter by ID
    pub async fn connect_adapter(&self, id: &str) -> Result<()> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(id).ok_or_else(|| {
            Error::not_found(format!("Adapter with ID '{}' not found", id))
        })?;
        
        adapter.connect().await
    }
    
    /// Disconnect a specific adapter by ID
    pub async fn disconnect_adapter(&self, id: &str) -> Result<()> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(id).ok_or_else(|| {
            Error::not_found(format!("Adapter with ID '{}' not found", id))
        })?;
        
        adapter.disconnect().await
    }
    
    /// Get the status of a specific adapter by ID
    pub async fn get_adapter_status(&self, id: &str) -> Result<ServiceAdapterStatus> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(id).ok_or_else(|| {
            Error::not_found(format!("Adapter with ID '{}' not found", id))
        })?;
        
        Ok(adapter.status().await)
    }
    
    /// Get the status of all registered adapters
    pub async fn get_all_adapter_statuses(&self) -> HashMap<String, ServiceAdapterStatus> {
        let adapters = self.adapters.read().await;
        let mut statuses = HashMap::new();
        
        for (id, adapter) in adapters.iter() {
            statuses.insert(id.clone(), adapter.status().await);
        }
        
        statuses
    }
    
    /// Initialize with default adapters for testing
    pub async fn with_test_adapters(mut self) -> Result<Self> {
        // Create and register test adapter
        let test_adapter = TestAdapter::new(self.event_bus.clone());
        self.register_adapter("test", test_adapter).await?;
        
        Ok(self)
    }
    
    /// Initialize with all available adapters
    pub async fn with_all_adapters(mut self) -> Result<Self> {
        // Create and register test adapter
        let test_adapter = TestAdapter::new(self.event_bus.clone());
        self.register_adapter("test", test_adapter).await?;
        
        // Create and register Twitch adapter
        let twitch_adapter = TwitchAdapter::new(self.event_bus.clone());
        self.register_adapter("twitch", twitch_adapter).await?;
        
        // OBS adapter will be implemented later
        // let obs_adapter = ObsAdapter::new(self.event_bus.clone());
        // self.register_adapter("obs", obs_adapter).await?;
        
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_service_lifecycle() {
        // Create service with test adapter
        let mut service = StreamService::new()
            .with_test_adapters()
            .await
            .expect("Failed to create service with test adapters");
        
        // Start service
        service.start(Some(9090)).await.expect("Failed to start service");
        
        // Verify adapter is connected
        let status = service.get_adapter_status("test").await.expect("Failed to get adapter status");
        assert_eq!(status, ServiceAdapterStatus::Connected);
        
        // Stop service
        service.stop().await.expect("Failed to stop service");
        
        // Verify adapter is disconnected
        let status = service.get_adapter_status("test").await.expect("Failed to get adapter status");
        assert_eq!(status, ServiceAdapterStatus::Disconnected);
    }
}