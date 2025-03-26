// src/adapters/test.rs
use crate::{
    adapters::{
        base::{AdapterConfig, BaseAdapter},
        common::{AdapterError, BackoffStrategy, RetryOptions, TraceHelper},
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
            return Err(AdapterError::config(
                "Interval must be between 100ms and 60000ms"
            ).into());
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
        // Record the callback registration in trace
        TraceHelper::record_adapter_operation(
            "test",
            "register_test_callback",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        let id = self.callbacks.register(callback).await;
        
        // Record successful registration in trace
        TraceHelper::record_adapter_operation(
            "test",
            "register_test_callback_success",
            Some(json!({
                "callback_id": id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        info!(callback_id = %id, "Registered test event callback");
        Ok(id)
    }

    /// Convert a JSON config to a TestConfig
    #[instrument(skip(config_json), level = "debug")]
    pub fn config_from_json(config_json: &Value) -> Result<TestConfig> {
        TestConfig::from_json(config_json)
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> TestConfig {
        self.config.read().await.clone()
    }
    
    /// Trigger a test event (useful for testing)
    pub async fn trigger_event(&self, event: TestEvent) -> Result<()> {
        // Get event type before we move the event
        let event_type = event.event_type();
        
        // Record the event trigger in trace
        TraceHelper::record_adapter_operation(
            "test",
            "trigger_event",
            Some(json!({
                "event_type": event_type,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        // Set up retry options for triggering
        let trigger_retry_options = RetryOptions::new(
            2, // Max attempts
            BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(100),
            },
            true, // Add jitter
        );
        
        // Clone the event for use in closure
        let event_clone = event.clone();
        
        // Use retry for triggering event
        let result = crate::adapters::common::with_retry(
            "trigger_test_event",
            trigger_retry_options,
            move |attempt| {
                let event_for_attempt = event_clone.clone();
                let callbacks = self.callbacks.clone();
                async move {
                    if attempt > 1 {
                        debug!(attempt, "Retrying test event trigger");
                    }
                    match callbacks.trigger(event_for_attempt).await {
                        Ok(count) => Ok(count),
                        Err(e) => Err(AdapterError::from_anyhow_error(
                            "event",
                            format!("Failed to trigger test event: {}", e),
                            e,
                        ))
                    }
                }
            }
        ).await;
        
        match result {
            Ok(count) => {
                debug!(callbacks = count, "Triggered test event successfully");
                
                // Record successful event trigger in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "trigger_event_success",
                    Some(json!({
                        "event_type": event_type,
                        "callbacks_triggered": count,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                Ok(())
            },
            Err(e) => {
                error!(error = %e, "Failed to trigger test event");
                
                // Record event trigger failure in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "trigger_event_failure",
                    Some(json!({
                        "event_type": event_type,
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                Err(e.into())
            }
        }
    }

    /// Generate test events at a regular interval
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn generate_events(
        &self,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<()> {
        // Record start of event generation in trace
        TraceHelper::record_adapter_operation(
            "test",
            "generate_events_start",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        // The callbacks registry is already wrapped in Arc, so we'll just clone it when needed
        
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

        // Set up retry options for event publishing
        let publish_retry_options = RetryOptions::new(
            3, // Max attempts
            BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(200),
            },
            true, // Add jitter
        );

        // Force publish multiple initial events to make sure something happens
        for i in 0..3 {
            let message = format!("Initial test event #{}", i);
            let payload = json!({
                "counter": i,
                "message": message.clone(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            // Use retry mechanism for publishing to event bus
            let event_type = "test.initial";
            let payload_clone = payload.clone();
            let publish_result = crate::adapters::common::with_retry(
                &format!("publish_initial_event_{}", i),
                publish_retry_options,
                move |attempt| {
                    let payload_for_attempt = payload_clone.clone();
                    async move {
                        if attempt > 1 {
                            debug!(attempt, "Retrying initial event publication");
                        }
                        match self.base.publish_event(event_type, payload_for_attempt).await {
                            Ok(receivers) => Ok(receivers),
                            Err(e) => Err(AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to publish initial event: {}", e),
                                e,
                            ))
                        }
                    }
                }
            ).await;
            
            match publish_result {
                Ok(receivers) => {
                    eprintln!("TEST ADAPTER: Published initial event #{} to {} receivers", i, receivers);
                    
                    // Record successful event publication in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "publish_initial_event_success",
                        Some(json!({
                            "counter": i,
                            "receivers": receivers,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                },
                Err(e) => {
                    eprintln!("TEST ADAPTER: Failed to publish initial event #{}: {}", i, e);
                    error!(error = %e, "Failed to publish initial event");
                    
                    // Record event publication failure in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "publish_initial_event_failure",
                        Some(json!({
                            "counter": i,
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                }
            }
            
            // Also trigger callbacks with retry
            let callback_event = TestEvent::Initial {
                counter: i,
                message: message.clone(),
                data: payload,
            };
            
            // Use retry for callback triggering
            let callback_event_clone = callback_event.clone();
            let callbacks_clone = self.callbacks.clone(); 
            let trigger_result = crate::adapters::common::with_retry(
                &format!("trigger_initial_callback_{}", i),
                publish_retry_options,
                move |attempt| {
                    let event_for_attempt = callback_event_clone.clone();
                    let callbacks_for_attempt = callbacks_clone.clone();
                    async move {
                        if attempt > 1 {
                            debug!(attempt, "Retrying initial callback trigger");
                        }
                        match callbacks_for_attempt.trigger(event_for_attempt).await {
                            Ok(count) => Ok(count),
                            Err(e) => Err(AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to trigger initial event callback: {}", e),
                                e,
                            ))
                        }
                    }
                }
            ).await;
            
            match trigger_result {
                Ok(count) => {
                    debug!(counter = i, callbacks = count, "Triggered initial test event callbacks");
                    
                    // Record successful callback triggering in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "trigger_initial_callback_success",
                        Some(json!({
                            "counter": i,
                            "callbacks_triggered": count,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                },
                Err(e) => {
                    error!(error = %e, "Failed to trigger initial test event callbacks");
                    
                    // Record callback triggering failure in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "trigger_initial_callback_failure",
                        Some(json!({
                            "counter": i,
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                }
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
                
                // Record shutdown in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "event_generator_shutdown",
                    Some(json!({
                        "reason": "shutdown_signal",
                        "events_generated": counter,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                break;
            }

            // Check if we're still connected - with more logging
            let connected = self.base.is_connected();
            eprintln!("TEST ADAPTER: Connection check: is_connected = {}", connected);
            
            if !connected {
                eprintln!("TEST ADAPTER: No longer connected, stopping event generator");
                info!("Test adapter no longer connected, stopping event generator");
                
                // Record disconnection in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "event_generator_shutdown",
                    Some(json!({
                        "reason": "disconnected",
                        "events_generated": counter,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                break;
            }

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

            // Use retry mechanism for standard event publishing
            let event_type = "test.event";
            let payload_clone = payload.clone();
            let publish_result = crate::adapters::common::with_retry(
                &format!("publish_standard_event_{}", counter),
                publish_retry_options,
                move |attempt| {
                    let payload_for_attempt = payload_clone.clone();
                    let counter_val = counter; // Capture counter for the debug log
                    async move {
                        if attempt > 1 {
                            debug!(attempt, counter = counter_val, "Retrying standard event publication");
                        }
                        match self.base.publish_event(event_type, payload_for_attempt).await {
                            Ok(receivers) => Ok(receivers),
                            Err(e) => Err(AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to publish standard event: {}", e),
                                e,
                            ))
                        }
                    }
                }
            ).await;
            
            match publish_result {
                Ok(receivers) => {
                    if receivers > 0 {
                        eprintln!("TEST ADAPTER: Published event #{} to {} receivers", counter, receivers);
                        info!(counter, receivers, "Successfully published test event to {} receivers", receivers);
                        
                        // Record successful standard event publication in trace
                        TraceHelper::record_adapter_operation(
                            "test",
                            "publish_standard_event_success",
                            Some(json!({
                                "counter": counter,
                                "receivers": receivers,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                    } else {
                        eprintln!("TEST ADAPTER: Published event #{} but no receivers", counter);
                        warn!(counter, "Published test event but no receivers were available");
                        
                        // Record no-receiver event publication in trace
                        TraceHelper::record_adapter_operation(
                            "test",
                            "publish_standard_event_no_receivers",
                            Some(json!({
                                "counter": counter,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                    }
                },
                Err(e) => {
                    eprintln!("TEST ADAPTER: Failed to publish event #{}: {}", counter, e);
                    error!(error = %e, counter, "Failed to publish test event");
                    
                    // Record event publication failure in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "publish_standard_event_failure",
                        Some(json!({
                            "counter": counter,
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                }
            }
            
            // Also trigger callbacks with retry
            let callback_event = TestEvent::Standard {
                counter,
                message: message.clone(),
                data: payload,
            };
            
            // Use retry for callback triggering
            let callback_event_clone = callback_event.clone();
            let counter_val = counter; // Capture counter for the debug log
            let callbacks_clone = self.callbacks.clone();
            let trigger_result = crate::adapters::common::with_retry(
                &format!("trigger_standard_callback_{}", counter),
                publish_retry_options,
                move |attempt| {
                    let event_for_attempt = callback_event_clone.clone();
                    let callbacks_for_attempt = callbacks_clone.clone();
                    async move {
                        if attempt > 1 {
                            debug!(attempt, counter = counter_val, "Retrying standard callback trigger");
                        }
                        match callbacks_for_attempt.trigger(event_for_attempt).await {
                            Ok(count) => Ok(count),
                            Err(e) => Err(AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to trigger standard event callback: {}", e),
                                e,
                            ))
                        }
                    }
                }
            ).await;
            
            match trigger_result {
                Ok(count) => {
                    debug!(counter, callbacks = count, "Triggered standard test event callbacks");
                    
                    // Record successful callback triggering in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "trigger_standard_callback_success",
                        Some(json!({
                            "counter": counter,
                            "callbacks_triggered": count,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                },
                Err(e) => {
                    error!(error = %e, counter, "Failed to trigger standard test event callbacks");
                    
                    // Record callback triggering failure in trace
                    TraceHelper::record_adapter_operation(
                        "test",
                        "trigger_standard_callback_failure",
                        Some(json!({
                            "counter": counter,
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                }
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

                // Use retry mechanism for special event publishing
                let special_event_type = "test.special";
                let special_payload_clone = special_payload.clone();
                let counter_val = counter; // Capture counter for debug log
                let special_result = crate::adapters::common::with_retry(
                    &format!("publish_special_event_{}", counter),
                    publish_retry_options,
                    move |attempt| {
                        let payload_for_attempt = special_payload_clone.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, counter = counter_val, "Retrying special event publication");
                            }
                            match self.base.publish_event(special_event_type, payload_for_attempt).await {
                                Ok(receivers) => Ok(receivers),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "event",
                                    format!("Failed to publish special event: {}", e),
                                    e,
                                ))
                            }
                        }
                    }
                ).await;
                
                match special_result {
                    Ok(receivers) => {
                        if receivers > 0 {
                            eprintln!("TEST ADAPTER: Published special event #{} to {} receivers", counter, receivers);
                            info!(counter, receivers, "Successfully published special test event to {} receivers", receivers);
                            
                            // Record successful special event publication in trace
                            TraceHelper::record_adapter_operation(
                                "test",
                                "publish_special_event_success",
                                Some(json!({
                                    "counter": counter,
                                    "receivers": receivers,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                        } else {
                            eprintln!("TEST ADAPTER: Published special event #{} but no receivers", counter);
                            warn!(counter, "Published special test event but no receivers were available");
                            
                            // Record no-receiver special event publication in trace
                            TraceHelper::record_adapter_operation(
                                "test",
                                "publish_special_event_no_receivers",
                                Some(json!({
                                    "counter": counter,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                        }
                    },
                    Err(e) => {
                        eprintln!("TEST ADAPTER: Failed to publish special event #{}: {}", counter, e);
                        error!(error = %e, counter, "Failed to publish special test event");
                        
                        // Record special event publication failure in trace
                        TraceHelper::record_adapter_operation(
                            "test",
                            "publish_special_event_failure",
                            Some(json!({
                                "counter": counter,
                                "error": e.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                    }
                }
                
                // Also trigger callbacks for special event with retry
                let special_callback_event = TestEvent::Special {
                    counter,
                    message: special_message.to_string(),
                    data: special_payload,
                };
                
                // Use retry for special callback triggering
                let special_callback_event_clone = special_callback_event.clone();
                let counter_val = counter; // Capture counter for debug log
                let special_trigger_result = crate::adapters::common::with_retry(
                    &format!("trigger_special_callback_{}", counter),
                    publish_retry_options,
                    move |attempt| {
                        let event_for_attempt = special_callback_event_clone.clone();
                        let callbacks_for_attempt = self.callbacks.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, counter = counter_val, "Retrying special callback trigger");
                            }
                            match callbacks_for_attempt.trigger(event_for_attempt).await {
                                Ok(count) => Ok(count),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "event",
                                    format!("Failed to trigger special event callback: {}", e),
                                    e,
                                ))
                            }
                        }
                    }
                ).await;
                
                match special_trigger_result {
                    Ok(count) => {
                        debug!(counter, callbacks = count, "Triggered special test event callbacks");
                        
                        // Record successful special callback triggering in trace
                        TraceHelper::record_adapter_operation(
                            "test",
                            "trigger_special_callback_success",
                            Some(json!({
                                "counter": counter,
                                "callbacks_triggered": count,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                    },
                    Err(e) => {
                        error!(error = %e, counter, "Failed to trigger special test event callbacks");
                        
                        // Record special callback triggering failure in trace
                        TraceHelper::record_adapter_operation(
                            "test",
                            "trigger_special_callback_failure",
                            Some(json!({
                                "counter": counter,
                                "error": e.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                    }
                }
            }

            counter += 1;
            sleep(Duration::from_millis(config.interval_ms)).await;
        }

        // Record event generator stop in trace
        TraceHelper::record_adapter_operation(
            "test",
            "generate_events_stop",
            Some(json!({
                "events_generated": counter,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;

        info!("Test event generator stopped");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TestAdapter {
    #[instrument(skip(self), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Record the operation in trace
        TraceHelper::record_adapter_operation(
            "test",
            "connect",
            Some(json!({
                "already_connected": self.base.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
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
                
                // Record error in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "generate_events_error",
                    Some(json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
            }
        });

        // Store the event handler task using BaseAdapter
        self.base.set_event_handler(handle).await;

        // Record successful connection in trace
        TraceHelper::record_adapter_operation(
            "test",
            "connect_success",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;

        info!("Test adapter connected and generating events");
        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Record the operation in trace
        TraceHelper::record_adapter_operation(
            "test",
            "disconnect",
            Some(json!({
                "currently_connected": self.base.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        // Only disconnect if connected
        if !self.base.is_connected() {
            debug!("Test adapter is already disconnected");
            return Ok(());
        }

        info!("Disconnecting test adapter");

        // Set disconnected state to stop event generation
        self.base.set_connected(false);

        // Stop the event handler (send shutdown signal and abort the task)
        match self.base.stop_event_handler().await {
            Ok(_) => {
                // Record successful disconnection in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "disconnect_success",
                    Some(json!({
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                info!("Test adapter disconnected");
                Ok(())
            },
            Err(e) => {
                // Record error in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "disconnect_error",
                    Some(json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                error!(error = %e, "Failed to stop test event generator");
                Err(AdapterError::from_anyhow_error(
                    "internal",
                    "Failed to stop event generator",
                    e
                ).into())
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }

    fn get_name(&self) -> &str {
        self.base.name()
    }

    #[instrument(skip(self, config), level = "debug")]
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        // Record the configuration operation in trace
        TraceHelper::record_adapter_operation(
            "test",
            "configure",
            Some(json!({
                "input_config": config,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        info!("Configuring test adapter");

        // Get the current configuration
        let current = self.config.read().await.clone();
        
        // Create a new config that starts with current values
        let mut new_config = current.clone();
        
        // Update only the fields that are provided in the input config
        if let Some(interval_ms) = config.get("interval_ms").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 100ms, maximum 60000ms)
            new_config.interval_ms = interval_ms.clamp(100, 60000);
        }
        
        if let Some(generate_special) = config.get("generate_special_events").and_then(|v| v.as_bool()) {
            new_config.generate_special_events = generate_special;
        }

        // Validate the new configuration
        match new_config.validate() {
            Ok(_) => {
                // Update our configuration
                let mut current_config = self.config.write().await;
                *current_config = new_config.clone();

                // Record successful configuration in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "configure_success",
                    Some(json!({
                        "new_config": {
                            "interval_ms": new_config.interval_ms,
                            "generate_special_events": new_config.generate_special_events,
                        },
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;

                info!(
                    interval_ms = new_config.interval_ms,
                    generate_special = new_config.generate_special_events,
                    "Test adapter configured"
                );

                Ok(())
            },
            Err(e) => {
                // Record configuration error in trace
                TraceHelper::record_adapter_operation(
                    "test",
                    "configure_error",
                    Some(json!({
                        "error": e.to_string(),
                        "attempted_config": {
                            "interval_ms": new_config.interval_ms,
                            "generate_special_events": new_config.generate_special_events,
                        },
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;

                error!(error = %e, "Invalid test adapter configuration");
                Err(AdapterError::from_anyhow_error(
                    "config",
                    "Invalid test adapter configuration",
                    e
                ).into())
            }
        }
    }
}

impl Clone for TestAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with the same event bus
        let _event_bus = self.base.event_bus();
        
        // IMPORTANT: This is a proper implementation of Clone that ensures callback integrity.
        // When an adapter is cloned, it's essential that all shared state wrapped in
        // Arc is properly cloned with Arc::clone to maintain the same underlying instances.
        //
        // Common mistakes fixed here:
        // 1. Creating a new configuration instead of sharing existing one
        // 2. Not sharing callback registry between clones
        // 3. Creating new RwLock instances instead of sharing existing ones
        //
        // The correct pattern is to use Arc::clone for ALL fields that contain
        // state that should be shared between clones:
        // 1. The config field must be shared so configuration changes affect all clones
        // 2. The callbacks registry must be shared so all callbacks are maintained
        // 3. Any other state containers should be properly shared with Arc::clone
        
        Self {
            base: self.base.clone(), // Use BaseAdapter's clone implementation
            config: Arc::clone(&self.config), // Share the same config instance
            callbacks: Arc::clone(&self.callbacks), // Share the same callback registry
        }
    }
}
