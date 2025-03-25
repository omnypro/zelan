// src/adapters/test.rs
use crate::{
    adapters::base::{AdapterConfig, BaseAdapter},
    EventBus, ServiceAdapter,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

// Import the callback registry for test events
use crate::adapters::test_callback::{TestCallbackRegistry, TestEvent};

/// A simple test adapter that generates events at regular intervals
/// Useful for testing the event bus and WebSocket server without external services
pub struct TestAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// Configuration specific to the TestAdapter
    config: Arc<RwLock<TestConfig>>,
    /// Callback registry for test events
    callbacks: Arc<TestCallbackRegistry>,
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
        if let Some(generate_special) = json
            .get("generate_special_events")
            .and_then(|v| v.as_bool())
        {
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
            return Err(anyhow::anyhow!(
                "Interval must be between 100ms and 60000ms"
            ));
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
            config: Arc::new(RwLock::new(TestConfig::default())),
            callbacks: Arc::new(TestCallbackRegistry::new()),
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
            config: Arc::new(RwLock::new(config)),
            callbacks: Arc::new(TestCallbackRegistry::new()),
        }
    }
    
    /// Register a callback for test events
    pub async fn register_test_callback<F>(&self, callback: F) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(TestEvent) -> Result<()> + Send + Sync + 'static,
    {
        let id = self.callbacks.register(callback).await;
        info!(callback_id = %id, "Registered test event callback");
        Ok(id)
    }

    /// Convert a JSON config to a TestConfig
    #[instrument(skip(config_json), level = "debug")]
    pub fn config_from_json(config_json: &Value) -> Result<TestConfig> {
        TestConfig::from_json(config_json)
    }

    /// Generate test events at a regular interval
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn generate_events(
        &self,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<()> {
        // Make a clone of the callbacks registry for use in the event generation loop
        let callbacks = Arc::clone(&self.callbacks);
        // Force a log entry to confirm execution
        eprintln!("TEST ADAPTER: Starting test event generator");
        info!("Starting test event generator");
        let mut counter = 0;
        
        // Adding multiple extra subscribers directly to make sure events have receivers
        // This ensures that there are always subscribers listening
        let receivers = vec![
            self.base.event_bus().subscribe(),
            self.base.event_bus().subscribe(),
            self.base.event_bus().subscribe(),
        ];
        eprintln!("TEST ADAPTER: Added {} direct subscribers to event bus", receivers.len());
        
        // Small initial delay to ensure full initialization
        sleep(Duration::from_millis(50)).await;

        // Print a debug message to confirm we're running
        eprintln!("TEST ADAPTER: Generator initialized, starting event loop");
        info!("Test event generator initialized, starting event loop");

        // Force publish multiple initial events to make sure something happens
        for i in 0..3 {
            let message = format!("Initial test event #{}", i);
            let payload = json!({
                "counter": i,
                "message": message.clone(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            // Publish to event bus
            if let Ok(receivers) = self.base.publish_event("test.initial", payload.clone()).await {
                eprintln!("TEST ADAPTER: Published initial event #{} to {} receivers", i, receivers);
            }
            
            // Also trigger callbacks
            let callback_event = TestEvent::Initial {
                counter: i,
                message: message.clone(),
                data: payload,
            };
            
            if let Err(e) = callbacks.trigger(callback_event).await {
                error!(error = %e, "Failed to trigger test initial event callbacks");
            } else {
                debug!(counter = i, "Triggered initial test event callbacks");
            }
            
            // Small delay between initial events
            sleep(Duration::from_millis(10)).await;
        }

        loop {
            // Check for shutdown signal
            let maybe_shutdown =
                tokio::time::timeout(tokio::time::Duration::from_millis(10), shutdown_rx.recv())
                    .await;

            if maybe_shutdown.is_ok() {
                eprintln!("TEST ADAPTER: Received shutdown signal");
                info!("Received shutdown signal for test event generator");
                break;
            }

            // Check if we're still connected - with more logging
            let connected = self.base.is_connected();
            eprintln!("TEST ADAPTER: Connection check: is_connected = {}", connected);
            
            if !connected {
                eprintln!("TEST ADAPTER: No longer connected, stopping event generator");
                info!("Test adapter no longer connected, stopping event generator");
                break;
            }
            
            // Always print the connection status on each loop iteration to help debug
            eprintln!("TEST ADAPTER: Connected = {} - continuing event generation", connected);
            info!("Test adapter connected: {}, continuing event generation", connected);

            // Get current config (read lock)
            let config = self.config.read().await.clone();

            // Generate a test event
            let message = format!("Test event #{}", counter);
            let payload = json!({
                "counter": counter,
                "message": message.clone(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            debug!(counter, "Publishing regular test event");

            // Use BaseAdapter to publish the event
            // Publish the event and capture the result
            let publish_result = self.base.publish_event("test.event", payload.clone()).await;
            match publish_result {
                Ok(receivers) => {
                    if receivers > 0 {
                        eprintln!("TEST ADAPTER: Published event #{} to {} receivers", counter, receivers);
                        info!(counter, receivers, "Successfully published test event to {} receivers", receivers);
                    } else {
                        eprintln!("TEST ADAPTER: Published event #{} but no receivers", counter);
                        warn!(counter, "Published test event but no receivers were available");
                    }
                },
                Err(e) => {
                    eprintln!("TEST ADAPTER: Failed to publish event #{}: {}", counter, e);
                    error!(error = %e, counter, "Failed to publish test event");
                }
            }
            
            // Also trigger callbacks
            let callback_event = TestEvent::Standard {
                counter,
                message: message.clone(),
                data: payload,
            };
            
            if let Err(e) = callbacks.trigger(callback_event).await {
                error!(error = %e, counter, "Failed to trigger test event callbacks");
            } else {
                debug!(counter, "Triggered test event callbacks");
            }

            // Generate a different event every 5 counts if enabled
            if config.generate_special_events && counter % 5 == 0 {
                let special_message = "This is a special test event";
                let special_payload = json!({
                    "counter": counter,
                    "special": true,
                    "message": special_message,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                debug!(counter, "Publishing special test event");

                // Publish the special event and capture the result
                let special_result = self.base.publish_event("test.special", special_payload.clone()).await;
                match special_result {
                    Ok(receivers) => {
                        if receivers > 0 {
                            eprintln!("TEST ADAPTER: Published special event #{} to {} receivers", counter, receivers);
                            info!(counter, receivers, "Successfully published special test event to {} receivers", receivers);
                        } else {
                            eprintln!("TEST ADAPTER: Published special event #{} but no receivers", counter);
                            warn!(counter, "Published special test event but no receivers were available");
                        }
                    },
                    Err(e) => {
                        eprintln!("TEST ADAPTER: Failed to publish special event #{}: {}", counter, e);
                        error!(error = %e, counter, "Failed to publish special test event");
                    }
                }
                
                // Also trigger callbacks for special event
                let callback_event = TestEvent::Special {
                    counter,
                    message: special_message.to_string(),
                    data: special_payload,
                };
                
                if let Err(e) = callbacks.trigger(callback_event).await {
                    error!(error = %e, counter, "Failed to trigger special test event callbacks");
                } else {
                    debug!(counter, "Triggered special test event callbacks");
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
        let handle = tauri::async_runtime::spawn(async move {
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
        // Create a new instance with the same event bus
        let _event_bus = self.base.event_bus();
        
        // IMPORTANT: Don't create a new config with hardcoded values - this would
        // break reactive configuration changes. Instead, share the same config lock.
        // This ensures all clones observe the same configuration state and callbacks
        // are maintained across clone boundaries.
        //
        // Previously, this was creating a new RwLock with hardcoded config values,
        // which broke reactive patterns as config changes didn't propagate to clones.
        
        Self {
            base: self.base.clone(), // Use BaseAdapter's clone implementation
            config: Arc::clone(&self.config), // Share the same config instance
            callbacks: Arc::clone(&self.callbacks), // Share the same callback registry
        }
    }
}
