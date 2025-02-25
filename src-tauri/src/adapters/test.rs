// src/adapters/test.rs
use crate::{EventBus, ServiceAdapter, StreamEvent};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// A simple test adapter that generates events at regular intervals
/// Useful for testing the event bus and WebSocket server without external services
pub struct TestAdapter {
    name: String,
    event_bus: Arc<EventBus>,
    connected: AtomicBool,
    task_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    config: tokio::sync::RwLock<TestAdapterConfig>,
}

/// Configuration for the test adapter
#[derive(Debug, Clone)]
struct TestAdapterConfig {
    /// Interval between events in milliseconds
    interval_ms: u64,
    /// Whether to generate special events
    generate_special_events: bool,
}

impl TestAdapter {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            name: "test".to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            task_handle: tokio::sync::Mutex::new(None),
            config: tokio::sync::RwLock::new(TestAdapterConfig {
                interval_ms: 1000, // Default: 1 second
                generate_special_events: true,
            }),
        }
    }

    /// Generate test events at a regular interval
    async fn generate_events(&self) -> Result<()> {
        let mut counter = 0;

        while self.connected.load(Ordering::SeqCst) {
            // Get current config (read lock)
            let config = self.config.read().await.clone();

            // Generate a test event
            let event = StreamEvent::new(
                "test",
                "test.event",
                json!({
                    "counter": counter,
                    "message": format!("Test event #{}", counter),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );

            // Publish the event
            if let Err(e) = self.event_bus.publish(event).await {
                eprintln!("Failed to publish test event: {}", e);
            }

            // Generate a different event every 5 counts if enabled
            if config.generate_special_events && counter % 5 == 0 {
                let event = StreamEvent::new(
                    "test",
                    "test.special",
                    json!({
                        "counter": counter,
                        "special": true,
                        "message": "This is a special test event",
                    }),
                );

                if let Err(e) = self.event_bus.publish(event).await {
                    eprintln!("Failed to publish special test event: {}", e);
                }
            }

            counter += 1;
            sleep(Duration::from_millis(config.interval_ms)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TestAdapter {
    async fn connect(&self) -> Result<()> {
        // Only connect if not already connected
        if self.connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Set connected state
        self.connected.store(true, Ordering::SeqCst);

        // Start generating events in a background task
        let self_clone = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.generate_events().await {
                eprintln!("Error in test event generator: {}", e);
            }
        });

        // Store the task handle
        *self.task_handle.lock().await = Some(handle);

        println!("Test adapter connected and generating events");
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        // Set disconnected state to stop event generation
        self.connected.store(false, Ordering::SeqCst);

        // Wait for the task to complete
        if let Some(handle) = self.task_handle.lock().await.take() {
            // We don't want to await the handle here as it might be waiting
            // for the connected flag to change, which we just did above
            handle.abort();
        }

        println!("Test adapter disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        // Update our configuration based on the provided JSON
        let mut current_config = self.config.write().await;

        // Extract the interval if provided
        if let Some(interval_ms) = config.get("interval_ms").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 100ms, maximum 60000ms)
            let interval = interval_ms.clamp(100, 60000);
            current_config.interval_ms = interval;
            println!("Test adapter interval set to: {}ms", interval);
        }

        // Extract the generate_special_events flag if provided
        if let Some(generate_special) = config
            .get("generate_special_events")
            .and_then(|v| v.as_bool())
        {
            current_config.generate_special_events = generate_special;
            println!("Test adapter special events: {}", generate_special);
        }

        println!("Test adapter configured with: {}", config);
        Ok(())
    }
}

impl Clone for TestAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with default config
        // This avoids blocking in clone which is bad practice
        Self {
            name: self.name.clone(),
            event_bus: Arc::clone(&self.event_bus),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            task_handle: tokio::sync::Mutex::new(None),
            config: tokio::sync::RwLock::new(TestAdapterConfig {
                interval_ms: 1000,             // Default value
                generate_special_events: true, // Default value
            }),
        }
    }
}
