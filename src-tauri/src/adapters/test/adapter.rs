// src/adapters/test/adapter.rs
use crate::{
    adapters::{
        base::{BaseAdapter, ServiceHelperImpl},
        common::{AdapterError, TraceHelper},
    },
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
use super::callback::{TestCallbackRegistry, TestEvent};

/// Configuration for the test adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Event generation interval in milliseconds
    #[serde(default = "default_interval_ms")]
    pub interval_ms: u64,
    /// Whether to simulate stream events
    #[serde(default = "default_simulate_stream")]
    pub simulate_stream: bool,
    /// Whether to generate errors periodically
    #[serde(default = "default_generate_errors")]
    pub generate_errors: bool,
    /// Auto-start event generation on connect
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,
}

/// Default interval in milliseconds
fn default_interval_ms() -> u64 {
    5000 // 5 seconds
}

/// Default simulate stream setting
fn default_simulate_stream() -> bool {
    true
}

/// Default generate errors setting
fn default_generate_errors() -> bool {
    false
}

/// Default auto-start setting
fn default_auto_start() -> bool {
    true
}

/// Default implementation for TestConfig
impl Default for TestConfig {
    fn default() -> Self {
        Self {
            interval_ms: default_interval_ms(),
            simulate_stream: default_simulate_stream(),
            generate_errors: default_generate_errors(),
            auto_start: default_auto_start(),
        }
    }
}

/// Implementation of AdapterConfig for TestConfig
impl crate::adapters::base::AdapterConfig for TestConfig {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn from_json(json: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(json.clone())?)
    }

    fn adapter_type() -> &'static str {
        "test"
    }

    fn validate(&self) -> anyhow::Result<()> {
        // No validation needed for test adapter
        Ok(())
    }
}

/// A simple test adapter that generates events at regular intervals
/// Useful for testing the event bus and WebSocket server without external services
pub struct TestAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// Configuration specific to the TestAdapter
    config: Arc<RwLock<TestConfig>>,
    /// Callback registry for test events
    callbacks: Arc<TestCallbackRegistry>,
    /// Event counter
    counter: Arc<RwLock<u64>>,
    /// Is event generation active?
    is_running: Arc<RwLock<bool>>,
    /// Is a simulated stream active?
    is_streaming: Arc<RwLock<bool>>,
    /// Service helper for implementing ServiceAdapter
    service_helper: ServiceHelperImpl,
}

#[async_trait]
impl ServiceAdapter for TestAdapter {
    /// Get the adapter type
    fn adapter_type(&self) -> &'static str {
        "test"
    }

    /// Connect the adapter
    async fn connect(&self) -> Result<(), AdapterError> {
        debug!("Test adapter connect() called");

        // Record the connect operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "connect",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // If auto-start is enabled, start event generation
        let config = self.config.read().await.clone();
        if config.auto_start {
            self.start_event_generation().await?;
        }

        Ok(())
    }

    /// Disconnect the adapter
    async fn disconnect(&self) -> Result<(), AdapterError> {
        debug!("Test adapter disconnect() called");

        // Stop event generation
        self.stop_event_generation().await?;

        // Record the disconnect operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "disconnect",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        Ok(())
    }

    /// Check the connection status
    async fn check_connection(&self) -> Result<bool, AdapterError> {
        // Test adapter is always "connected" if it's running
        Ok(*self.is_running.read().await)
    }

    /// Get the adapter status
    async fn get_status(&self) -> Result<Value, AdapterError> {
        let is_running = *self.is_running.read().await;
        let counter = *self.counter.read().await;
        let is_streaming = *self.is_streaming.read().await;
        let config = self.config.read().await.clone();

        let status = json!({
            "adapter_type": self.adapter_type(),
            "name": self.base.name(),
            "id": self.base.id(),
            "is_running": is_running,
            "counter": counter,
            "is_streaming": is_streaming,
            "config": {
                "interval_ms": config.interval_ms,
                "simulate_stream": config.simulate_stream,
                "generate_errors": config.generate_errors,
                "auto_start": config.auto_start,
            }
        });

        Ok(status)
    }

    /// Execute a command
    async fn execute_command(
        &self,
        command: &str,
        args: Option<&Value>,
    ) -> Result<Value, AdapterError> {
        match command {
            "status" => self.get_status().await,
            "start" => {
                self.start_event_generation().await?;
                Ok(json!({"status": "started"}))
            }
            "stop" => {
                self.stop_event_generation().await?;
                Ok(json!({"status": "stopped"}))
            }
            "reset_counter" => {
                *self.counter.write().await = 0;
                Ok(json!({"status": "counter_reset"}))
            }
            "set_interval" => {
                if let Some(args) = args {
                    if let Some(interval) = args.get("interval_ms") {
                        if let Some(interval) = interval.as_u64() {
                            self.config.write().await.interval_ms = interval;
                            Ok(json!({"status": "interval_updated", "interval_ms": interval}))
                        } else {
                            Err(AdapterError::config(
                                "interval_ms must be a positive integer",
                            ))
                        }
                    } else {
                        Err(AdapterError::config(
                            "Missing required argument: interval_ms",
                        ))
                    }
                } else {
                    Err(AdapterError::config("Missing required arguments"))
                }
            }
            "start_stream" => {
                *self.is_streaming.write().await = true;

                // Publish stream.online event
                self.base
                    .event_bus()
                    .publish(crate::StreamEvent::new(
                        self.adapter_type(),
                        "stream.online",
                        json!({
                            "id": format!("test_stream_{}", chrono::Utc::now().timestamp()),
                            "title": "Test Stream",
                            "started_at": chrono::Utc::now().to_rfc3339(),
                        }),
                    ))
                    .await?;

                Ok(json!({"status": "stream_started"}))
            }
            "stop_stream" => {
                *self.is_streaming.write().await = false;

                // Publish stream.offline event
                self.base
                    .event_bus()
                    .publish(crate::StreamEvent::new(
                        self.adapter_type(),
                        "stream.offline",
                        json!({
                            "id": format!("test_stream_{}", chrono::Utc::now().timestamp()),
                            "ended_at": chrono::Utc::now().to_rfc3339(),
                        }),
                    ))
                    .await?;

                Ok(json!({"status": "stream_stopped"}))
            }
            "generate_event" => {
                if let Some(args) = args {
                    if let Some(event_type) = args.get("type").and_then(|v| v.as_str()) {
                        let data = args.get("data").cloned().unwrap_or(json!({}));

                        // Publish custom event
                        self.base
                            .event_bus()
                            .publish(crate::StreamEvent::new(
                                self.adapter_type(),
                                event_type,
                                data.clone(),
                            ))
                            .await?;

                        Ok(json!({"status": "event_generated", "type": event_type}))
                    } else {
                        Err(AdapterError::config("Missing required argument: type"))
                    }
                } else {
                    Err(AdapterError::config("Missing required arguments"))
                }
            }
            _ => Err(AdapterError::invalid_command(format!(
                "Unknown command: {}",
                command
            ))),
        }
    }

    /// Handle lifecycle events
    async fn handle_lifecycle_event(
        &self,
        event: &str,
        _data: Option<&Value>,
    ) -> Result<(), AdapterError> {
        match event {
            "initialize" => {
                // Already handled in constructor
                Ok(())
            }
            _ => {
                // Ignore unknown events
                Ok(())
            }
        }
    }

    /// Get a feature by key from service helper
    async fn get_feature(&self, key: &str) -> Result<Option<String>, AdapterError> {
        self.service_helper.get_feature(key).await
    }
}

impl TestAdapter {
    /// Parse configuration from JSON
    pub fn config_from_json(json_value: &Value) -> Result<TestConfig, AdapterError> {
        serde_json::from_value(json_value.clone())
            .map_err(|e| AdapterError::config(format!("Failed to parse TestAdapter config: {}", e)))
    }

    /// Create a new test adapter
    pub fn new(name: &str, id: &str, event_bus: Arc<EventBus>, config: Option<TestConfig>) -> Self {
        // Create default config if none provided
        let config = config.unwrap_or_else(|| TestConfig {
            interval_ms: default_interval_ms(),
            simulate_stream: default_simulate_stream(),
            generate_errors: default_generate_errors(),
            auto_start: default_auto_start(),
        });

        // Create callback registry
        let callbacks = Arc::new(TestCallbackRegistry::new());

        // Create base adapter
        let base = BaseAdapter::new(
            name,
            id,
            event_bus.clone(),
            serde_json::to_value(config.clone()).unwrap_or_default(),
            Arc::new(crate::auth::token_manager::TokenManager::new()),
        );

        // Create service helper
        let service_helper = ServiceHelperImpl::new(Arc::new(base.clone()));

        Self {
            base,
            config: Arc::new(RwLock::new(config)),
            callbacks,
            counter: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            is_streaming: Arc::new(RwLock::new(false)),
            service_helper,
        }
    }

    /// Get the base adapter
    pub fn base(&self) -> &BaseAdapter {
        &self.base
    }

    /// Get the adapter name
    pub fn name(&self) -> String {
        self.base.name().to_string()
    }

    /// Get the adapter ID
    pub fn id(&self) -> String {
        self.base.id().to_string()
    }

    /// Create a new test adapter with the provided configuration
    pub fn with_config(name: &str, id: &str, event_bus: Arc<EventBus>, config: TestConfig) -> Self {
        Self::new(name, id, event_bus, Some(config))
    }

    /// Register a callback for test events
    pub async fn register_callback<F>(
        &self,
        callback: F,
    ) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(TestEvent) -> Result<()> + Send + Sync + 'static,
    {
        Ok(self.callbacks.register(callback).await)
    }

    /// Start event generation
    pub async fn start_event_generation(&self) -> Result<(), AdapterError> {
        // Skip if already running
        if *self.is_running.read().await {
            debug!("Test adapter event generation already running");
            return Ok(());
        }

        debug!("Starting test adapter event generation");

        // Mark as running
        *self.is_running.write().await = true;

        // Record the start operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "start_event_generation",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // Clone for use in async block
        let base = self.base.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let counter = Arc::clone(&self.counter);
        let is_running = Arc::clone(&self.is_running);
        let is_streaming = Arc::clone(&self.is_streaming);
        let config_arc = Arc::clone(&self.config);
        let adapter_type = self.adapter_type().to_string();

        // Spawn event generation task
        tokio::spawn(async move {
            debug!("Event generation task started");

            while *is_running.read().await {
                // Get the current interval setting
                let config = config_arc.read().await.clone();
                let interval = Duration::from_millis(config.interval_ms);
                let simulate_stream = config.simulate_stream;
                let generate_errors = config.generate_errors;

                // Increment counter
                let current_counter = {
                    let mut counter_guard = counter.write().await;
                    *counter_guard += 1;
                    *counter_guard
                };

                debug!("Generating test event: counter={}", current_counter);

                // Create event data
                let event_data = json!({
                    "counter": current_counter,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "is_streaming": *is_streaming.read().await,
                });

                // Determine if this is a special event (every 5 counts)
                let is_special = current_counter % 5 == 0;
                let event_type = if is_special {
                    "special_event"
                } else {
                    "standard_event"
                };

                // Generate error if enabled and counter is divisible by 7
                if generate_errors && current_counter % 7 == 0 {
                    error!("Generated test error: counter={}", current_counter);

                    // Trigger error event callback
                    let _ = callbacks
                        .trigger(TestEvent::Error {
                            counter: current_counter,
                            message: format!("Test error at count {}", current_counter),
                            data: event_data.clone(),
                        })
                        .await;

                    // Publish error event
                    let _ = base
                        .event_bus()
                        .publish(crate::StreamEvent::new(
                            &adapter_type,
                            "error_event",
                            event_data.clone(),
                        ))
                        .await;
                } else {
                    // Normal event processing
                    if is_special {
                        // Trigger special event callback
                        let _ = callbacks
                            .trigger(TestEvent::Special {
                                counter: current_counter,
                                message: format!("Special test event {}", current_counter),
                                data: event_data.clone(),
                            })
                            .await;
                    } else {
                        // Trigger standard event callback
                        let _ = callbacks
                            .trigger(TestEvent::Standard {
                                counter: current_counter,
                                message: format!("Standard test event {}", current_counter),
                                data: event_data.clone(),
                            })
                            .await;
                    }

                    // Publish event to event bus
                    let _ = base
                        .event_bus()
                        .publish(crate::StreamEvent::new(
                            &adapter_type,
                            event_type,
                            event_data,
                        ))
                        .await;
                }

                // Simulate stream events if enabled and counter hits specific values
                if simulate_stream {
                    let is_currently_streaming = *is_streaming.read().await;

                    // Start stream at counter 3
                    if current_counter == 3 && !is_currently_streaming {
                        *is_streaming.write().await = true;

                        // Publish stream.online event
                        let _ = base
                            .event_bus()
                            .publish(crate::StreamEvent::new(
                                &adapter_type,
                                "stream.online",
                                json!({
                                    "id": format!("test_stream_{}", current_counter),
                                    "title": "Test Stream",
                                    "started_at": chrono::Utc::now().to_rfc3339(),
                                }),
                            ))
                            .await;
                    }
                    // Stop stream at counter 8
                    else if current_counter == 8 && is_currently_streaming {
                        *is_streaming.write().await = false;

                        // Publish stream.offline event
                        let _ = base
                            .event_bus()
                            .publish(crate::StreamEvent::new(
                                &adapter_type,
                                "stream.offline",
                                json!({
                                    "id": format!("test_stream_{}", 3), // Use the same ID as when we started
                                    "ended_at": chrono::Utc::now().to_rfc3339(),
                                }),
                            ))
                            .await;
                    }
                }

                // Wait for next interval
                sleep(interval).await;
            }

            debug!("Event generation task stopped");
        });

        Ok(())
    }

    /// Stop event generation
    pub async fn stop_event_generation(&self) -> Result<(), AdapterError> {
        // Skip if not running
        if !*self.is_running.read().await {
            debug!("Test adapter event generation not running");
            return Ok(());
        }

        debug!("Stopping test adapter event generation");

        // Mark as not running
        *self.is_running.write().await = false;

        // Record the stop operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "stop_event_generation",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        Ok(())
    }
}

// Implement Clone for TestAdapter
impl Clone for TestAdapter {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            config: Arc::clone(&self.config),
            callbacks: Arc::clone(&self.callbacks),
            counter: Arc::clone(&self.counter),
            is_running: Arc::clone(&self.is_running),
            is_streaming: Arc::clone(&self.is_streaming),
            service_helper: self.service_helper.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    pub use crate::adapters::tests::test_test::*;
}
