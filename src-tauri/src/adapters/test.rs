// src/adapters/test.rs
use crate::{adapters::base::{AdapterConfig, BaseAdapter}, EventBus, ServiceAdapter};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

/// A simple test adapter that generates events at regular intervals
/// Useful for testing the event bus and WebSocket server without external services
pub struct TestAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// Configuration specific to the TestAdapter
    config: RwLock<TestConfig>,
}

/// Configuration for the test adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Interval between events in milliseconds
    pub interval_ms: u64,
    /// Whether to generate special events
    pub generate_special_events: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            interval_ms: 1000, // Default: 1 second
            generate_special_events: true,
        }
    }
}

impl AdapterConfig for TestConfig {
    fn to_json(&self) -> Value {
        json!({
            "interval_ms": self.interval_ms,
            "generate_special_events": self.generate_special_events,
        })
    }
    
    fn from_json(json: &Value) -> Result<Self> {
        let mut config = TestConfig::default();
        
        // Extract the interval if provided
        if let Some(interval_ms) = json.get("interval_ms").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 100ms, maximum 60000ms)
            config.interval_ms = interval_ms.clamp(100, 60000);
        }
        
        // Extract the generate_special_events flag if provided
        if let Some(generate_special) = json.get("generate_special_events").and_then(|v| v.as_bool()) {
            config.generate_special_events = generate_special;
        }
        
        Ok(config)
    }
    
    fn adapter_type() -> &'static str {
        "test"
    }
    
    fn validate(&self) -> Result<()> {
        // Ensure interval is reasonable
        if self.interval_ms < 100 || self.interval_ms > 60000 {
            return Err(anyhow::anyhow!("Interval must be between 100ms and 60000ms"));
        }
        
        Ok(())
    }
}

impl TestAdapter {
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new test adapter");
        Self {
            base: BaseAdapter::new("test", event_bus),
            config: RwLock::new(TestConfig::default()),
        }
    }
    
    #[instrument(skip(event_bus), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: TestConfig) -> Self {
        info!(
            interval_ms = config.interval_ms,
            generate_special = config.generate_special_events,
            "Creating test adapter with custom config"
        );
        Self {
            base: BaseAdapter::new("test", event_bus),
            config: RwLock::new(config),
        }
    }
    
    /// Convert a JSON config to a TestConfig
    #[instrument(skip(config_json), level = "debug")]
    pub fn config_from_json(config_json: &Value) -> Result<TestConfig> {
        TestConfig::from_json(config_json)
    }

    /// Generate test events at a regular interval
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn generate_events(&self, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>) -> Result<()> {
        info!("Starting test event generator");
        let mut counter = 0;

        loop {
            // Check for shutdown signal
            let maybe_shutdown = tokio::time::timeout(
                tokio::time::Duration::from_millis(10), 
                shutdown_rx.recv()
            ).await;
            
            if maybe_shutdown.is_ok() {
                info!("Received shutdown signal for test event generator");
                break;
            }
            
            // Check if we're still connected
            if !self.base.is_connected() {
                info!("Test adapter no longer connected, stopping event generator");
                break;
            }
            
            // Get current config (read lock)
            let config = self.config.read().await.clone();

            // Generate a test event
            let payload = json!({
                "counter": counter,
                "message": format!("Test event #{}", counter),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            debug!(counter, "Publishing regular test event");
            
            // Use BaseAdapter to publish the event
            if let Err(e) = self.base.publish_event("test.event", payload).await {
                error!(error = %e, counter, "Failed to publish test event");
            }

            // Generate a different event every 5 counts if enabled
            if config.generate_special_events && counter % 5 == 0 {
                let special_payload = json!({
                    "counter": counter,
                    "special": true,
                    "message": "This is a special test event",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                debug!(counter, "Publishing special test event");
                
                if let Err(e) = self.base.publish_event("test.special", special_payload).await {
                    error!(error = %e, counter, "Failed to publish special test event");
                }
            }

            counter += 1;
            sleep(Duration::from_millis(config.interval_ms)).await;
        }

        info!("Test event generator stopped");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TestAdapter {
    #[instrument(skip(self), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Only connect if not already connected
        if self.base.is_connected() {
            info!("Test adapter is already connected");
            return Ok(());
        }

        info!("Connecting test adapter");
        
        // Create the shutdown channel
        let (_, shutdown_rx) = self.base.create_shutdown_channel().await;
        
        // Set connected state
        self.base.set_connected(true);

        // Start generating events in a background task
        let self_clone = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.generate_events(shutdown_rx).await {
                error!(error = %e, "Error in test event generator");
            }
        });

        // Store the event handler task using BaseAdapter
        self.base.set_event_handler(handle).await;

        info!("Test adapter connected and generating events");
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if connected
        if !self.base.is_connected() {
            debug!("Test adapter is already disconnected");
            return Ok(());
        }
        
        info!("Disconnecting test adapter");
        
        // Set disconnected state to stop event generation
        self.base.set_connected(false);

        // Stop the event handler (send shutdown signal and abort the task)
        self.base.stop_event_handler().await?;

        info!("Test adapter disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }

    fn get_name(&self) -> &str {
        self.base.name()
    }

    #[instrument(skip(self, config), level = "debug")]
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        info!("Configuring test adapter");
        
        // Parse the config using our AdapterConfig trait implementation
        let new_config = TestConfig::from_json(&config)?;
        
        // Validate the new configuration
        new_config.validate()?;
        
        // Update our configuration
        let mut current_config = self.config.write().await;
        *current_config = new_config.clone();
        
        info!(
            interval_ms = new_config.interval_ms,
            generate_special = new_config.generate_special_events,
            "Test adapter configured"
        );
        
        Ok(())
    }
}

impl Clone for TestAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with the same event bus and config
        // This avoids blocking in clone which is bad practice
        let event_bus = self.base.event_bus();
        
        // We have to assume a default config here without blocking
        Self {
            base: BaseAdapter::new("test", event_bus),
            config: RwLock::new(TestConfig::default()),
        }
    }
}
