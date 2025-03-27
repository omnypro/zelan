// Base adapter implementation with common functionality
use crate::{EventBus, StreamEvent};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

// Constants shared across adapters
pub const SHUTDOWN_CHANNEL_SIZE: usize = 1;

/// Base trait for adapter configuration
pub trait AdapterConfig: Clone + Send + Sync + std::fmt::Debug {
    /// Convert the configuration to a JSON value for storage
    fn to_json(&self) -> serde_json::Value;

    /// Create a configuration from a JSON value
    fn from_json(json: &serde_json::Value) -> Result<Self>
    where
        Self: Sized;

    /// Get a string identifying the adapter type
    fn adapter_type() -> &'static str
    where
        Self: Sized;

    /// Validate the configuration
    fn validate(&self) -> Result<()>;
}

/// Base adapter implementation with common functionality
pub struct BaseAdapter {
    /// Name of the adapter
    name: String,
    /// Unique identifier for the adapter
    id: String,
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Whether the adapter is currently connected
    connected: AtomicBool,
    /// Shutdown signal channel
    shutdown_signal: Mutex<Option<mpsc::Sender<()>>>,
    /// Event handler task
    event_handler: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// Last error that occurred
    last_error: Mutex<Option<String>>,
    /// Adapter configuration as raw JSON
    config: Mutex<serde_json::Value>,
    /// Token manager for authentication
    token_manager: Arc<crate::auth::TokenManager>,
}

impl Clone for BaseAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with the same name and event bus
        // IMPORTANT: We must maintain the proper pattern of sharing immutable state
        // through Arc and using atomic operations for shared mutable state.
        Self {
            name: self.name.clone(),
            id: self.id.clone(),
            event_bus: Arc::clone(&self.event_bus),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            shutdown_signal: Mutex::new(None), // Don't clone the shutdown channel - each clone manages its own tasks
            event_handler: Mutex::new(None), // Don't clone the task handle - each clone manages its own tasks
            last_error: Mutex::new(None), // Don't clone the last error - each clone tracks its own errors
            config: Mutex::new(self.config.blocking_lock().clone()), // Clone the config JSON
            token_manager: Arc::clone(&self.token_manager), // Share the token manager
        }
    }
}

impl BaseAdapter {
    /// Create a new base adapter
    #[instrument(skip(event_bus, config_value, token_manager), level = "debug")]
    pub fn new(
        name: &str,
        id: &str,
        event_bus: Arc<EventBus>,
        config_value: serde_json::Value,
        token_manager: Arc<crate::auth::TokenManager>,
    ) -> Self {
        info!(adapter = %name, id = %id, "Creating new adapter");
        Self {
            name: name.to_string(),
            id: id.to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            shutdown_signal: Mutex::new(None),
            event_handler: Mutex::new(None),
            last_error: Mutex::new(None),
            config: Mutex::new(config_value),
            token_manager,
        }
    }

    /// Get the name of the adapter
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the ID of the adapter
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get a reference to the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Save the adapter configuration
    #[instrument(skip(self), level = "debug")]
    pub async fn save_config(&self) -> Result<(), crate::adapters::common::AdapterError> {
        debug!(adapter = %self.name, "Saving adapter configuration");
        // For now, just a stub implementation
        Ok(())
    }

    /// Check if the adapter is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Set the connected state
    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::SeqCst);
    }

    /// Store the event handler task
    #[instrument(skip(self, handle), level = "debug")]
    pub async fn set_event_handler(&self, handle: tauri::async_runtime::JoinHandle<()>) {
        debug!(adapter = %self.name, "Storing event handler task");
        *self.event_handler.lock().await = Some(handle);
    }

    /// Store the shutdown signal sender
    #[instrument(skip(self, sender), level = "debug")]
    pub async fn set_shutdown_signal(&self, sender: mpsc::Sender<()>) {
        debug!(adapter = %self.name, "Storing shutdown signal sender");
        *self.shutdown_signal.lock().await = Some(sender);
    }

    /// Create a shutdown channel
    #[instrument(skip(self), level = "debug")]
    pub async fn create_shutdown_channel(&self) -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
        debug!(adapter = %self.name, "Creating shutdown channel");
        let (shutdown_tx, shutdown_rx) = mpsc::channel(SHUTDOWN_CHANNEL_SIZE);
        self.set_shutdown_signal(shutdown_tx.clone()).await;
        (shutdown_tx, shutdown_rx)
    }

    /// Start a periodic polling operation
    #[instrument(skip(self, poll_fn), fields(adapter = %self.name), level = "debug")]
    pub async fn start_polling<F, Fut>(
        &self,
        poll_fn: Box<F>,
        interval: Duration,
    ) -> Result<(), crate::adapters::common::AdapterError>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), crate::adapters::common::AdapterError>>
            + Send
            + 'static,
    {
        debug!(adapter = %self.name, interval_ms = %interval.as_millis(), "Starting polling operation");

        // Create shutdown channel
        let (_, mut shutdown_rx) = self.create_shutdown_channel().await;

        // Clone what we need for the task
        let name = self.name.clone();

        // Create a task for polling
        let handle = tauri::async_runtime::spawn(async move {
            debug!(adapter = %name, "Polling task started");

            // Create the interval timer
            let mut interval_timer = tokio::time::interval(interval);

            // Run until shutdown
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        debug!(adapter = %name, "Polling task received shutdown signal");
                        break;
                    }

                    // Wait for interval tick
                    _ = interval_timer.tick() => {
                        // Execute the poll function
                        match poll_fn().await {
                            Ok(_) => {
                                debug!(adapter = %name, "Poll operation completed successfully");
                            }
                            Err(e) => {
                                warn!(adapter = %name, error = %e, "Error during poll operation");
                            }
                        }
                    }
                }
            }

            debug!(adapter = %name, "Polling task exiting");
        });

        // Store the task handle
        self.set_event_handler(handle).await;

        debug!(adapter = %self.name, "Polling operation started");
        Ok(())
    }

    /// Stop polling operation
    #[instrument(skip(self), level = "debug")]
    pub async fn stop_polling(&self) -> Result<(), crate::adapters::common::AdapterError> {
        debug!(adapter = %self.name, "Stopping polling operation");
        self.stop_event_handler().await.map_err(|e| {
            crate::adapters::common::AdapterError::internal(format!(
                "Failed to stop event handler: {}",
                e
            ))
        })
    }

    /// Send shutdown signal and stop event handler
    #[instrument(skip(self), level = "debug")]
    pub async fn stop_event_handler(&self) -> Result<()> {
        if let Some(shutdown_sender) = self.shutdown_signal.lock().await.take() {
            debug!(adapter = %self.name, "Sending shutdown signal");
            let _ = shutdown_sender.send(()).await;
            // Small delay to allow shutdown signal to be processed
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        } else {
            debug!(adapter = %self.name, "No shutdown signal channel available");
        }

        // Cancel the event handler task if it's still running
        if let Some(handle) = self.event_handler.lock().await.take() {
            debug!(adapter = %self.name, "Aborting event handler task");
            handle.abort();
        } else {
            debug!(adapter = %self.name, "No event handler task to abort");
        }

        Ok(())
    }

    /// Publish an event to the event bus
    #[instrument(skip(self, payload), level = "debug")]
    pub async fn publish_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<usize> {
        debug!(
            adapter = %self.name,
            event_type = %event_type,
            "Publishing event"
        );

        // Create trace context for tracking this event's journey
        let mut trace = crate::flow::TraceContext::new(self.name.clone(), event_type.to_string());

        // Add the initial span for adapter publishing - use the correct component name "TestHarness" for test adapter
        let component_name = if self.name == "test" {
            "TestHarness"
        } else {
            &self.name
        };
        trace
            .add_span("create", component_name)
            .context(Some(serde_json::json!({
                "adapter_connected": self.is_connected(),
                "event_type": event_type
            })));

        // Get the global trace registry to record our trace
        let registry = crate::flow::get_trace_registry();

        // Complete the span and the trace
        trace.complete_span();
        trace.complete();

        // Record the trace in the registry
        registry.record_trace(trace.clone()).await;

        // Create the event with a new trace context (for the event's journey)
        let mut event_trace =
            crate::flow::TraceContext::new(self.name.clone(), event_type.to_string());
        event_trace
            .add_span("create", &self.name)
            .context(Some(serde_json::json!({
                "adapter_connected": self.is_connected(),
                "event_type": event_type
            })));

        let stream_event =
            StreamEvent::new_with_trace(&self.name, event_type, payload, event_trace);

        match self.event_bus.publish(stream_event).await {
            Ok(receivers) => {
                debug!(
                    adapter = %self.name,
                    event_type = %event_type,
                    receivers = receivers,
                    "Event published successfully"
                );
                Ok(receivers)
            }
            Err(e) => {
                error!(
                    adapter = %self.name,
                    event_type = %event_type,
                    error = %e,
                    "Failed to publish event"
                );
                Err(e.into())
            }
        }
    }

    /// Get the current adapter settings from the service
    /// This is a helper method to access adapter settings through the event bus
    #[instrument(skip(self), level = "debug")]
    pub async fn get_adapter_settings(&self) -> Result<crate::AdapterSettings> {
        // Use a system event to request settings
        let payload = serde_json::json!({
            "adapter": self.name,
            "action": "get_settings",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Create a custom event to get settings
        let stream_event = StreamEvent::new("system", "adapter.get_settings", payload);

        // Publish event through the event bus
        match self.event_bus.publish(stream_event).await {
            Ok(_) => {
                debug!(adapter = %self.name, "Successfully requested adapter settings");

                // Return default settings for now - in real implementation,
                // we would wait for a response
                Ok(crate::AdapterSettings {
                    enabled: true,
                    config: serde_json::json!({}),
                    display_name: self.name.clone(),
                    description: format!("Settings for {} adapter", self.name),
                })
            }
            Err(e) => {
                error!(adapter = %self.name, error = %e, "Failed to request adapter settings");
                Err(e.into())
            }
        }
    }

    /// Update adapter settings in the service
    /// This is a helper method to update adapter settings through the event bus
    #[instrument(skip(self, settings), level = "debug")]
    pub async fn update_adapter_settings(&self, settings: crate::AdapterSettings) -> Result<()> {
        // Use a system event to update settings
        let payload = serde_json::json!({
            "adapter": self.name,
            "action": "update_settings",
            "settings": settings,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Create a custom event to update settings
        let stream_event = StreamEvent::new("system", "adapter.update_settings", payload);

        // Publish event through the event bus
        match self.event_bus.publish(stream_event).await {
            Ok(_) => {
                debug!(adapter = %self.name, "Successfully requested adapter settings update");
                Ok(())
            }
            Err(e) => {
                error!(adapter = %self.name, error = %e, "Failed to update adapter settings");
                Err(e.into())
            }
        }
    }

    /// Set the last error message
    #[instrument(skip(self, error), level = "debug")]
    pub async fn set_last_error(&self, error: impl std::fmt::Display) {
        let error_str = error.to_string();
        error!(adapter = %self.name, error = %error_str, "Adapter error occurred");
        *self.last_error.lock().await = Some(error_str);
    }

    /// Get the last error message
    pub async fn get_last_error(&self) -> Option<String> {
        self.last_error.lock().await.clone()
    }

    /// Clear the last error message
    pub async fn clear_last_error(&self) {
        *self.last_error.lock().await = None;
    }

    /// Record the start of a standard adapter operation with trace
    pub async fn record_operation_start(
        &self,
        operation: &str,
        context: Option<serde_json::Value>,
    ) {
        use crate::adapters::common::TraceHelper;

        // Create default context if none provided
        let context = context.unwrap_or_else(|| {
            serde_json::json!({
                "adapter": self.name,
                "connected": self.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        });

        TraceHelper::record_adapter_operation(
            &self.name,
            &format!("{}_start", operation),
            Some(context),
        )
        .await;
    }

    /// Record the success of a standard adapter operation with trace
    pub async fn record_operation_success(
        &self,
        operation: &str,
        context: Option<serde_json::Value>,
    ) {
        use crate::adapters::common::TraceHelper;

        // Create default context if none provided
        let context = context.unwrap_or_else(|| {
            serde_json::json!({
                "adapter": self.name,
                "connected": self.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        });

        TraceHelper::record_adapter_operation(
            &self.name,
            &format!("{}_success", operation),
            Some(context),
        )
        .await;
    }

    /// Record the failure of a standard adapter operation with trace
    pub async fn record_operation_failure(
        &self,
        operation: &str,
        error: &impl std::fmt::Display,
        context: Option<serde_json::Value>,
    ) {
        use crate::adapters::common::TraceHelper;

        // Create default context with error information
        let mut context_map = match context {
            Some(ctx) => ctx,
            None => serde_json::json!({
                "adapter": self.name,
                "connected": self.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };

        // Add error info to context
        if let serde_json::Value::Object(ref mut map) = context_map {
            map.insert(
                "error".to_string(),
                serde_json::Value::String(error.to_string()),
            );
        }

        TraceHelper::record_adapter_operation(
            &self.name,
            &format!("{}_failure", operation),
            Some(context_map),
        )
        .await;
    }

    /// Publish a connection status event
    pub async fn publish_connection_event(
        &self,
        status: &str,
        context: Option<serde_json::Value>,
    ) -> Result<usize> {
        let event_type = format!("connection.{}", status);

        // Create default context if none provided
        let context = context.unwrap_or_else(|| {
            serde_json::json!({
                "adapter": self.name,
                "connected": self.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        });

        self.publish_event(&event_type, context).await
    }

    /// Publish a connection established event
    pub async fn publish_connection_established(
        &self,
        details: Option<serde_json::Value>,
    ) -> Result<usize> {
        self.publish_connection_event("established", details).await
    }

    /// Publish a connection failure event
    pub async fn publish_connection_failed(
        &self,
        error: &impl std::fmt::Display,
        details: Option<serde_json::Value>,
    ) -> Result<usize> {
        // Create the error context
        let mut context = details.unwrap_or_else(|| {
            serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        });

        // Add error info to context
        if let serde_json::Value::Object(ref mut map) = context {
            map.insert(
                "error".to_string(),
                serde_json::Value::String(error.to_string()),
            );
        }

        self.publish_connection_event("failed", Some(context)).await
    }

    /// Publish a connection closed event
    pub async fn publish_connection_closed(
        &self,
        details: Option<serde_json::Value>,
    ) -> Result<usize> {
        self.publish_connection_event("closed", details).await
    }
}

/// Helper struct that provides default implementations for ServiceAdapter methods
/// This is designed to be used as a field in adapter implementations
#[derive(Clone)]
pub struct ServiceHelperImpl {
    /// Base adapter reference
    base: Arc<BaseAdapter>,
    /// Features supported by this helper
    features: Vec<String>,
}

impl ServiceHelperImpl {
    /// Create a new service helper
    pub fn new(base: Arc<BaseAdapter>) -> Self {
        Self {
            base,
            features: vec![],
        }
    }

    /// Get a reference to the base adapter
    pub fn base(&self) -> &BaseAdapter {
        &self.base
    }

    /// Get features supported by this helper
    pub async fn get_features(&self) -> Vec<String> {
        self.features.clone()
    }

    /// Add a feature to this helper
    pub fn add_feature(&mut self, feature: &str) {
        self.features.push(feature.to_string());
    }

    /// Check if a feature is supported
    pub async fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&feature.to_string())
    }

    /// Get a feature by key - default implementation
    pub async fn get_feature(
        &self,
        _key: &str,
    ) -> Result<Option<String>, crate::adapters::common::AdapterError> {
        Ok(None)
    }

    /// Default implementation for checking connection
    pub async fn check_connection(&self) -> Result<bool, crate::adapters::common::AdapterError> {
        Ok(self.base.is_connected())
    }

    /// Register for recovery with the recovery manager
    pub async fn register_for_recovery(
        &self,
        _recovery_data: serde_json::Value,
    ) -> Result<(), crate::adapters::common::AdapterError> {
        // TODO: Implementation to register with recovery manager
        Ok(())
    }

    /// Default implementation for disconnect method
    ///
    /// This method handles the standard disconnection flow:
    /// 1. Records the operation start in trace
    /// 2. Checks if already disconnected
    /// 3. Sets the disconnected state
    /// 4. Stops the event handler
    /// 5. Publishes a disconnection event
    /// 6. Records disconnect success in trace
    pub async fn disconnect(
        &self,
        cleanup_fn: Option<
            Box<
                dyn Fn() -> std::pin::Pin<
                        Box<
                            dyn std::future::Future<
                                    Output = Result<(), crate::adapters::common::AdapterError>,
                                > + Send,
                        >,
                    > + Send
                    + Sync,
            >,
        >,
    ) -> Result<(), crate::adapters::common::AdapterError> {
        // Record the disconnect operation in trace
        self.base
            .record_operation_start(
                "disconnect",
                Some(serde_json::json!({
                    "currently_connected": self.base.is_connected(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

        // Only disconnect if currently connected
        if !self.base.is_connected() {
            debug!("Adapter {} already disconnected", self.base.name());
            return Ok(());
        }

        info!("Disconnecting {} adapter", self.base.name());

        // Set disconnected state
        self.base.set_connected(false);

        // Stop the event handler
        match self.base.stop_event_handler().await {
            Ok(_) => debug!(
                "Successfully stopped event handler for {}",
                self.base.name()
            ),
            Err(e) => {
                warn!(error = %e, "Issues while stopping event handler for {}", self.base.name())
            }
        }

        // Run adapter-specific cleanup if provided
        if let Some(cleanup) = cleanup_fn {
            if let Err(e) = cleanup().await {
                warn!(error = %e, "Error during cleanup for {}", self.base.name());
            }
        }

        // Publish disconnection event
        let event_payload = serde_json::json!({
            "message": format!("Disconnected from {} adapter", self.base.name()),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Err(e) = self
            .base
            .publish_connection_closed(Some(event_payload.clone()))
            .await
        {
            warn!(error = %e, "Failed to publish disconnection event for {}", self.base.name());
        }

        // Record disconnect success
        self.base
            .record_operation_success(
                "disconnect",
                Some(serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

        info!("Disconnected from {} adapter", self.base.name());
        Ok(())
    }

    /// Default implementation for get_status method
    pub async fn get_status(
        &self,
        adapter_type: &str,
    ) -> Result<serde_json::Value, crate::adapters::common::AdapterError> {
        Ok(serde_json::json!({
            "adapter_type": adapter_type,
            "name": self.base.name(),
            "is_connected": self.base.is_connected(),
        }))
    }

    /// Default implementation for handle_lifecycle_event method
    pub async fn handle_lifecycle_event(
        &self,
        event: &str,
        _data: Option<&serde_json::Value>,
    ) -> Result<(), crate::adapters::common::AdapterError> {
        debug!("Ignoring lifecycle event: {}", event);
        Ok(())
    }

    /// Default implementation for execute_command method
    pub async fn execute_command(
        &self,
        command: &str,
        _args: Option<&serde_json::Value>,
        adapter_type: &str,
    ) -> Result<serde_json::Value, crate::adapters::common::AdapterError> {
        match command {
            "status" => self.get_status(adapter_type).await,
            _ => Err(crate::adapters::common::AdapterError::invalid_command(
                format!("Unknown command: {}", command),
            )),
        }
    }
}
