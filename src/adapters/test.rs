//! Test adapter implementation
//!
//! This module provides a test adapter that generates events at regular intervals
//! for testing purposes.

use crate::adapters::ServiceAdapter;
use crate::core::EventBus;
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
#[derive(Debug)]
pub struct TestAdapter {
    /// Name of the adapter
    name: String,
    /// Connected status
    connected: Arc<RwLock<bool>>,
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Configuration specific to the TestAdapter
    config: Arc<RwLock<TestConfig>>,
    /// Task handle for the event generator
    event_handler: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Shutdown sender for stopping the event generator
    shutdown_sender: Arc<RwLock<Option<tokio::sync::mpsc::Sender<()>>>>,
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

impl TestAdapter {
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new test adapter");
        Self {
            name: "test".to_string(),
            connected: Arc::new(RwLock::new(false)),
            event_bus,
            config: Arc::new(RwLock::new(TestConfig::default())),
            event_handler: Arc::new(RwLock::new(None)),
            shutdown_sender: Arc::new(RwLock::new(None)),
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
            name: "test".to_string(),
            connected: Arc::new(RwLock::new(false)),
            event_bus,
            config: Arc::new(RwLock::new(config)),
            event_handler: Arc::new(RwLock::new(None)),
            shutdown_sender: Arc::new(RwLock::new(None)),
        }
    }

    /// Convert a JSON config to a TestConfig
    #[instrument(skip(config_json), level = "debug")]
    pub fn config_from_json(config_json: &Value) -> Result<TestConfig> {
        let mut config = TestConfig::default();

        // Extract the interval if provided
        if let Some(interval_ms) = config_json.get("interval_ms").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 100ms, maximum 60000ms)
            config.interval_ms = interval_ms.clamp(100, 60000);
        }

        // Extract the generate_special_events flag if provided
        if let Some(generate_special) = config_json
            .get("generate_special_events")
            .and_then(|v| v.as_bool())
        {
            config.generate_special_events = generate_special;
        }

        Ok(config)
    }

    /// Send an event to the event bus
    async fn publish_event(&self, event_type: &str, payload: Value) -> Result<usize> {
        let event = crate::core::StreamEvent::new("test", event_type, payload);
        self.event_bus.publish(event).await.map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Generate test events at a regular interval
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn generate_events(
        self: Arc<Self>,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<()> {
        // Force a log entry to confirm execution
        eprintln!("TEST ADAPTER: Starting test event generator");
        info!("Starting test event generator");
        let mut counter = 0;
        
        // Adding multiple extra subscribers directly to make sure events have receivers
        // This ensures that there are always subscribers listening
        let receivers = vec![
            self.event_bus.subscribe(),
            self.event_bus.subscribe(),
            self.event_bus.subscribe(),
        ];
        eprintln!("TEST ADAPTER: Added {} direct subscribers to event bus", receivers.len());
        
        // Small initial delay to ensure full initialization
        sleep(Duration::from_millis(50)).await;

        // Print a debug message to confirm we're running
        eprintln!("TEST ADAPTER: Generator initialized, starting event loop");
        info!("Test event generator initialized, starting event loop");

        // Force publish multiple initial events to make sure something happens
        for i in 0..3 {
            let payload = json!({
                "counter": i,
                "message": format!("Initial test event #{}", i),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            if let Ok(receivers) = self.publish_event("test.initial", payload).await {
                eprintln!("TEST ADAPTER: Published initial event #{} to {} receivers", i, receivers);
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
            let connected = *self.connected.read().await;
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
            let payload = json!({
                "counter": counter,
                "message": format!("Test event #{}", counter),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            debug!(counter, "Publishing regular test event");

            // Publish the event and capture the result
            let publish_result = self.publish_event("test.event", payload).await;
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

            // Generate a different event every 5 counts if enabled
            if config.generate_special_events && counter % 5 == 0 {
                let special_payload = json!({
                    "counter": counter,
                    "special": true,
                    "message": "This is a special test event",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                debug!(counter, "Publishing special test event");

                // Publish the special event and capture the result
                let special_result = self.publish_event("test.special", special_payload).await;
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
        if *self.connected.read().await {
            info!("Test adapter is already connected");
            return Ok(());
        }

        info!("Connecting test adapter");

        // Create the shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
        *self.shutdown_sender.write().await = Some(shutdown_tx);

        // Set connected state
        *self.connected.write().await = true;

        // Start generating events in a background task
        let self_clone = Arc::new(self.clone());
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.generate_events(shutdown_rx).await {
                error!(error = %e, "Error in test event generator");
            }
        });

        // Store the event handler task
        *self.event_handler.write().await = Some(handle);

        info!("Test adapter connected and generating events");
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if connected
        if !*self.connected.read().await {
            debug!("Test adapter is already disconnected");
            return Ok(());
        }

        info!("Disconnecting test adapter");

        // Set disconnected state to stop event generation
        *self.connected.write().await = false;

        // Send shutdown signal if possible
        if let Some(tx) = self.shutdown_sender.write().await.take() {
            let _ = tx.send(()).await;
            debug!("Sent shutdown signal to test event generator");
        }

        // Abort the task if it's still running
        if let Some(handle) = self.event_handler.write().await.take() {
            handle.abort();
            debug!("Aborted test event generator task");
        }

        info!("Test adapter disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        std::future::block_on(async {
            *self.connected.read().await
        })
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    #[instrument(skip(self, config), level = "debug")]
    async fn configure(&self, config: Value) -> Result<()> {
        info!("Configuring test adapter");

        // Parse the config
        let new_config = Self::config_from_json(&config)?;

        // Validate the new configuration
        if new_config.interval_ms < 100 || new_config.interval_ms > 60000 {
            return Err(anyhow::anyhow!(
                "Interval must be between 100ms and 60000ms"
            ));
        }

        // Update our configuration
        *self.config.write().await = new_config.clone();

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
        Self {
            name: self.name.clone(),
            connected: self.connected.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
            event_handler: self.event_handler.clone(),
            shutdown_sender: self.shutdown_sender.clone(),
        }
    }
}