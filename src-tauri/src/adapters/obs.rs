// src/adapters/obs.rs
use crate::{
    adapters::{
        base::{AdapterConfig, BaseAdapter},
        common::{AdapterError, BackoffStrategy, RetryOptions, TraceHelper},
    },
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use obws::{client::ConnectConfig, requests::EventSubscription, Client};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::async_runtime::{Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, instrument, warn, Instrument};

// Import the callback registry for OBS events
use crate::adapters::obs_callback::{ObsCallbackRegistry, ObsEvent};

// Default connection settings
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 4455;
const DEFAULT_PASSWORD: &str = "";
const RECONNECT_DELAY_MS: u64 = 5000;
// SHUTDOWN_CHANNEL_SIZE not needed anymore since we use BaseAdapter

/// Configuration for the OBS adapter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObsConfig {
    /// The host to connect to OBS WebSocket
    pub host: String,
    /// The port for OBS WebSocket
    pub port: u16,
    /// The password for OBS WebSocket (if required)
    pub password: String,
    /// Whether to auto-connect on startup
    pub auto_connect: bool,
    /// Whether to gather additional scene details on scene changes
    pub include_scene_details: bool,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            password: DEFAULT_PASSWORD.to_string(),
            auto_connect: true,
            include_scene_details: true,
        }
    }
}

impl AdapterConfig for ObsConfig {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "host": self.host,
            "port": self.port,
            "password": self.password,
            "auto_connect": self.auto_connect,
            "include_scene_details": self.include_scene_details,
        })
    }

    fn from_json(json: &serde_json::Value) -> Result<Self> {
        let mut config = ObsConfig::default();
        
        // Extract host if provided
        if let Some(host) = json.get("host").and_then(|v| v.as_str()) {
            config.host = host.to_string();
        }
        
        // Extract port if provided
        if let Some(port) = json.get("port").and_then(|v| v.as_u64()) {
            config.port = port as u16;
        }
        
        // Extract password if provided
        if let Some(password) = json.get("password").and_then(|v| v.as_str()) {
            config.password = password.to_string();
        }
        
        // Extract auto_connect if provided
        if let Some(auto_connect) = json.get("auto_connect").and_then(|v| v.as_bool()) {
            config.auto_connect = auto_connect;
        }
        
        // Extract include_scene_details if provided
        if let Some(include_scene_details) = json.get("include_scene_details").and_then(|v| v.as_bool()) {
            config.include_scene_details = include_scene_details;
        }
        
        Ok(config)
    }
    
    fn adapter_type() -> &'static str {
        "obs"
    }
    
    fn validate(&self) -> Result<()> {
        // Ensure port is in a reasonable range
        if self.port == 0 {
            return Err(AdapterError::config("Port cannot be 0").into());
        }
        
        // Host cannot be empty
        if self.host.is_empty() {
            return Err(AdapterError::config("Host cannot be empty").into());
        }
        
        Ok(())
    }
}

/// OBS Studio adapter for connecting to OBS via its WebSocket API
pub struct ObsAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// OBS WebSocket client
    client: Arc<Mutex<Option<Arc<Client>>>>,
    /// Configuration specific to the OBS adapter
    config: Arc<RwLock<ObsConfig>>,
    /// Callback registry for OBS events
    callbacks: Arc<ObsCallbackRegistry>,
}

impl ObsAdapter {
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new OBS adapter");
        Self {
            base: BaseAdapter::new("obs", event_bus),
            client: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(ObsConfig::default())),
            callbacks: Arc::new(ObsCallbackRegistry::new()),
        }
    }

    /// Creates a new OBS adapter with the given configuration
    #[instrument(skip(event_bus), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: ObsConfig) -> Self {
        info!(
            host = %config.host,
            port = config.port,
            auto_connect = config.auto_connect,
            "Creating OBS adapter with custom config"
        );
        Self {
            base: BaseAdapter::new("obs", event_bus),
            client: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(config)),
            callbacks: Arc::new(ObsCallbackRegistry::new()),
        }
    }
    
    /// Register a callback for OBS events
    pub async fn register_obs_callback<F>(&self, callback: F) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(ObsEvent) -> Result<()> + Send + Sync + 'static,
    {
        // Record the callback registration in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "register_obs_callback",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        let id = self.callbacks.register(callback).await;
        
        // Record successful registration in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "register_obs_callback_success",
            Some(json!({
                "callback_id": id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        info!(callback_id = %id, "Registered OBS event callback");
        Ok(id)
    }
    
    /// Get the current configuration (useful for testing)
    pub async fn get_config(&self) -> ObsConfig {
        self.config.read().await.clone()
    }
    
    /// Trigger an OBS event (useful for testing)
    pub async fn trigger_event(&self, event: ObsEvent) -> Result<()> {
        // Get event type before we move the event
        let event_type = event.event_type();
        
        // Record the event trigger in trace
        TraceHelper::record_adapter_operation(
            "obs",
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
            "trigger_obs_event",
            trigger_retry_options,
            move |attempt| {
                let event_for_attempt = event_clone.clone();
                let callbacks = self.callbacks.clone();
                async move {
                    if attempt > 1 {
                        debug!(attempt, "Retrying OBS event trigger");
                    }
                    match callbacks.trigger(event_for_attempt).await {
                        Ok(count) => Ok(count),
                        Err(e) => Err(AdapterError::from_anyhow_error(
                            "event",
                            format!("Failed to trigger OBS event: {}", e),
                            e,
                        ))
                    }
                }
            }
        ).await;
        
        match result {
            Ok(count) => {
                debug!(callbacks = count, "Triggered OBS event successfully");
                
                // Record successful event trigger in trace
                TraceHelper::record_adapter_operation(
                    "obs",
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
                error!(error = %e, "Failed to trigger OBS event");
                
                // Record event trigger failure in trace
                TraceHelper::record_adapter_operation(
                    "obs",
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

    /// Handles incoming OBS events and converts them to StreamEvents
    #[instrument(
        skip(client, event_bus, config, connected, shutdown_rx, callbacks),
        level = "debug"
    )]
    async fn handle_obs_events(
        client: Arc<Client>,
        event_bus: Arc<EventBus>,
        config: ObsConfig,
        connected: Arc<AtomicBool>,
        mut shutdown_rx: mpsc::Receiver<()>,
        callbacks: Arc<ObsCallbackRegistry>,
    ) -> Result<()> {
        // Record start of event handling in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "handle_events_start",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
    
        // Set up OBS event listener
        let events = match client.events() {
            Ok(events) => events,
            Err(e) => {
                error!(error = %e, "Failed to create OBS event stream");
                
                // Record error in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "event_stream_creation_error",
                    Some(json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                return Err(AdapterError::connection_with_source(
                    format!("Failed to create OBS event stream: {}", e),
                    e
                ).into());
            }
        };

        pin_mut!(events);
        info!("OBS event listener started");

        // Continue as long as connected is true and no shutdown signal is received
        loop {
            // Listen for the next event with a timeout to allow for checking connected state
            let maybe_event = tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal for OBS event handler");
                    
                    // Record shutdown in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "event_handler_shutdown",
                        Some(json!({
                            "reason": "shutdown_signal",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                    
                    break;
                }

                // Check connection status periodically
                _ = sleep(Duration::from_secs(5)) => {
                    if !connected.load(Ordering::SeqCst) {
                        info!("Connection flag set to false, stopping OBS event handler");
                        
                        // Record disconnection in trace
                        TraceHelper::record_adapter_operation(
                            "obs",
                            "event_handler_shutdown",
                            Some(json!({
                                "reason": "disconnected",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        break;
                    }
                    debug!("Connection check passed, OBS event handler still active");
                    continue;
                }

                // Try to get the next event
                ev = events.next() => ev,
            };

            if !connected.load(Ordering::SeqCst) {
                info!("Connection flag set to false, stopping OBS event handler");
                break;
            }

            // Handle next event from the stream
            match maybe_event {
                Some(event) => {
                    // Convert OBS event to our internal format
                    match event {
                        obws::events::Event::CurrentProgramSceneChanged { id } => {
                            let scene_name = id.name.clone();
                            debug!(scene = %scene_name, "OBS scene changed");
                            
                            // Record scene change in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "scene_changed",
                                Some(json!({
                                    "scene_name": scene_name,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;

                            let mut payload = json!({
                                "scene_name": scene_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            // If configured to include scene details, gather them
                            if config.include_scene_details {
                                debug!("Fetching additional scene details");
                                
                                // Set up retry options for fetching scene details
                                let scene_details_retry = RetryOptions::new(
                                    2, // Max attempts
                                    BackoffStrategy::Exponential {
                                        base_delay: Duration::from_millis(10),
                                        max_delay: Duration::from_millis(100),
                                    },
                                    true, // Add jitter
                                );
                                
                                // Get a clone of the client for the closure
                                let client_clone = client.clone();
                                
                                // Use retry for fetching scene details
                                let scene_details_result = crate::adapters::common::with_retry(
                                    "fetch_scene_details",
                                    scene_details_retry,
                                    move |attempt| {
                                        let client_for_attempt = client_clone.clone();
                                        let scene_name_for_attempt = id.name.clone();
                                        async move {
                                            if attempt > 1 {
                                                debug!(attempt, "Retrying scene details fetch");
                                            }
                                            match client_for_attempt.scenes().list().await {
                                                Ok(scene_list) => {
                                                    if let Some(scene) = scene_list.scenes.iter()
                                                        .find(|s| s.id.name == scene_name_for_attempt)
                                                    {
                                                        Ok(Some(scene.index))
                                                    } else {
                                                        Ok(None)
                                                    }
                                                },
                                                Err(e) => Err(AdapterError::from_anyhow_error(
                                                    "api",
                                                    format!("Failed to fetch scene details: {}", e),
                                                    anyhow::anyhow!("{}", e)
                                                ))
                                            }
                                        }
                                    }
                                ).await;
                                
                                // Process the result
                                match scene_details_result {
                                    Ok(Some(scene_index)) => {
                                        payload["scene_index"] = json!(scene_index);
                                        debug!(scene_index, "Added scene details");
                                    },
                                    Ok(None) => {
                                        debug!("Scene not found in scene list");
                                    },
                                    Err(e) => {
                                        warn!(error = %e, "Failed to fetch scene details after retries");
                                    }
                                }
                            }

                            // Set up retry options for publishing events
                            let publish_retry = RetryOptions::new(
                                2, // Max attempts
                                BackoffStrategy::Exponential {
                                    base_delay: Duration::from_millis(10),
                                    max_delay: Duration::from_millis(100),
                                },
                                true, // Add jitter
                            );
                            
                            // Use retry for publishing event
                            let event_bus_clone = event_bus.clone();
                            let payload_clone = payload.clone();
                            let publish_result = crate::adapters::common::with_retry(
                                "publish_scene_change",
                                publish_retry,
                                move |attempt| {
                                    let event_bus_for_attempt = event_bus_clone.clone();
                                    let payload_for_attempt = payload_clone.clone();
                                    async move {
                                        if attempt > 1 {
                                            debug!(attempt, "Retrying scene change event publication");
                                        }
                                        let stream_event = StreamEvent::new("obs", "scene.changed", payload_for_attempt);
                                        match event_bus_for_attempt.publish(stream_event).await {
                                            Ok(receivers) => Ok(receivers),
                                            Err(e) => Err(AdapterError::from_anyhow_error(
                                                "event",
                                                format!("Failed to publish scene change event: {}", e),
                                                anyhow::anyhow!("{}", e)
                                            ))
                                        }
                                    }
                                }
                            ).await;
                            
                            // Process publish result
                            match publish_result {
                                Ok(receivers) => {
                                    debug!(scene = %scene_name, receivers, "Published scene change event");
                                },
                                Err(e) => {
                                    error!(error = %e, "Failed to publish OBS scene change event");
                                }
                            }
                            
                            // Also trigger callbacks with retry
                            let callback_event = ObsEvent::SceneChanged { 
                                scene_name: scene_name.clone(),
                                data: payload,
                            };
                            
                            let callbacks_clone = callbacks.clone();
                            let callback_event_clone = callback_event.clone();
                            let trigger_result = crate::adapters::common::with_retry(
                                "trigger_scene_change_callbacks",
                                publish_retry,
                                move |attempt| {
                                    let callbacks_for_attempt = callbacks_clone.clone();
                                    let event_for_attempt = callback_event_clone.clone();
                                    async move {
                                        if attempt > 1 {
                                            debug!(attempt, "Retrying scene change callback trigger");
                                        }
                                        match callbacks_for_attempt.trigger(event_for_attempt).await {
                                            Ok(count) => Ok(count),
                                            Err(e) => Err(AdapterError::from_anyhow_error(
                                                "event",
                                                format!("Failed to trigger scene change callbacks: {}", e),
                                                anyhow::anyhow!("{}", e)
                                            ))
                                        }
                                    }
                                }
                            ).await;
                            
                            // Process trigger result
                            match trigger_result {
                                Ok(count) => {
                                    debug!(scene = %scene_name, callbacks = count, "Triggered scene change callbacks");
                                },
                                Err(e) => {
                                    error!(error = %e, "Failed to trigger OBS scene change callbacks");
                                }
                            }
                        }
                        obws::events::Event::SceneItemEnableStateChanged {
                            scene,
                            item_id,
                            enabled,
                            ..
                        } => {
                            debug!(
                                scene = ?scene,
                                item_id = ?item_id,
                                enabled = enabled,
                                "Scene item visibility changed"
                            );

                            let payload = json!({
                                "scene_name": scene,
                                "scene_item_id": item_id,
                                "enabled": enabled,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "scene_item.visibility_changed", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS scene item event");
                            }
                        }
                        obws::events::Event::StreamStateChanged { active, .. } => {
                            let event_type = if active {
                                "stream.started"
                            } else {
                                "stream.stopped"
                            };

                            info!(
                                active = active,
                                event_type = %event_type,
                                "OBS stream state changed"
                            );

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload.clone());
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS stream state event");
                            }
                            
                            // Also trigger callbacks
                            let callback_event = ObsEvent::StreamStateChanged { 
                                active,
                                data: payload,
                            };
                            
                            if let Err(e) = callbacks.trigger(callback_event).await {
                                error!(error = %e, "Failed to trigger OBS stream state callbacks");
                            } else {
                                debug!(active = active, "Triggered stream state callbacks");
                            }
                        }
                        obws::events::Event::RecordStateChanged { active, .. } => {
                            let event_type = if active {
                                "recording.started"
                            } else {
                                "recording.stopped"
                            };

                            info!(
                                active = active,
                                event_type = %event_type,
                                "OBS recording state changed"
                            );

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS recording state event");
                            }
                        }
                        // Provide connection events
                        obws::events::Event::ExitStarted => {
                            warn!("OBS is shutting down");

                            let payload = json!({
                                "message": "OBS is shutting down",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "connection.closing", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS exit event");
                            }

                            // OBS is shutting down, so we should stop
                            connected.store(false, Ordering::SeqCst);
                            break;
                        }
                        // Catch all other events and log them with generic handling
                        _ => {
                            // We can add specific handling for more event types as needed
                            let event_name = format!("{:?}", event);
                            debug!(event = %event_name, "Received other OBS event");

                            // Create a simple payload with event details
                            let payload = json!({
                                "event_type": event_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", "event.generic", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish generic OBS event");
                            }
                        }
                    }
                }
                // In obws 0.14 the events stream returns Event directly, not Result<Event, Error>
                // This branch is kept for future compatibility or in case errors are added
                None => {
                    // End of events stream usually means connection closed
                    info!("OBS events stream ended");

                    // If we're still supposed to be connected, this is unexpected
                    if connected.load(Ordering::SeqCst) {
                        warn!("OBS WebSocket connection closed unexpectedly while still enabled");

                        // Send a disconnection event
                        let payload = json!({
                            "message": "OBS WebSocket connection closed unexpectedly",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        let stream_event = StreamEvent::new("obs", "connection.closed", payload);
                        if let Err(e) = event_bus.publish(stream_event).await {
                            error!(error = %e, "Failed to publish OBS disconnection event");
                        }

                        // We're no longer connected
                        connected.store(false, Ordering::SeqCst);
                    } else {
                        debug!("OBS events stream ended normally (adapter was disabled)");
                    }

                    break;
                }
            }
        }

        info!("OBS event handler stopped");
        Ok(())
    }

    /// Get current scenes and publish an initial state
    #[instrument(skip(client, event_bus), level = "debug")]
    async fn publish_initial_state(client: &Arc<Client>, event_bus: &Arc<EventBus>) -> Result<()> {
        info!("Publishing initial OBS state");

        // Get version information
        match client.general().version().await {
            Ok(version) => {
                info!(
                    obs_version = %version.obs_version,
                    websocket_version = %version.obs_web_socket_version,
                    platform = %version.platform,
                    "OBS system info"
                );

                let payload = json!({
                    "obs_version": version.obs_version,
                    "websocket_version": version.obs_web_socket_version,
                    "platform": version.platform,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "system.info", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish OBS version info");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch OBS version info");
            }
        }

        // Get current scene
        match client.scenes().current_program_scene().await {
            Ok(current_scene) => {
                info!(scene = ?current_scene, "Current scene");

                let payload = json!({
                    "scene_name": current_scene,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "scene.current", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish current scene");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch current scene");
            }
        }

        // Get all scenes
        match client.scenes().list().await {
            Ok(scenes) => {
                debug!(scene_count = scenes.scenes.len(), "Retrieved scene list");

                let payload = json!({
                    "scenes": scenes.scenes,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "scenes.list", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish scenes list");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch scenes list");
            }
        }

        // Get stream status
        match client.streaming().status().await {
            Ok(status) => {
                info!(active = status.active, "Stream status");

                let payload = json!({
                    "active": status.active,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "stream.status", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish stream status");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch stream status");
            }
        }

        // Get recording status
        match client.recording().status().await {
            Ok(status) => {
                info!(active = status.active, "Recording status");

                let payload = json!({
                    "active": status.active,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "recording.status", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish recording status");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch recording status");
            }
        }

        info!("Initial OBS state published");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for ObsAdapter {
    #[instrument(skip(self), fields(adapter = "obs"), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Record the connect operation in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "connect",
            Some(json!({
                "already_connected": self.base.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
    
        // Check if already connected
        if self.base.is_connected() {
            info!("OBS adapter is already connected");
            return Ok(());
        }

        // Get connection configuration
        let config = self.config.read().await.clone();

        // Connect to OBS WebSocket server
        info!(
            host = %config.host,
            port = config.port,
            "Connecting to OBS WebSocket"
        );

        // Set up retry options for OBS connection
        let connect_retry_options = RetryOptions::new(
            3, // Max attempts
            BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(5),
            },
            true, // Add jitter
        );
        
        // Log the connection parameters
        debug!(
            host = %config.host,
            port = config.port,
            password_provided = !config.password.is_empty(),
            "Preparing OBS connection parameters"
        );

        // Use retry for connecting to OBS - we need to recreate the connect_opts in the closure
        // because ConnectConfig doesn't implement Clone
        let config_clone = config.clone();
        let connect_result = crate::adapters::common::with_retry(
            "connect_to_obs",
            connect_retry_options,
            move |attempt| {
                // Recreate connection options for each attempt since it doesn't implement Clone
                let config_for_attempt = config_clone.clone();
                async move {
                    if attempt > 1 {
                        debug!(attempt, "Retrying OBS connection");
                    }
                    
                    // Recreate the connection options for each attempt
                    let connect_opts = ConnectConfig {
                        host: config_for_attempt.host.clone(),
                        port: config_for_attempt.port,
                        password: if config_for_attempt.password.is_empty() {
                            None
                        } else {
                            Some(config_for_attempt.password.clone())
                        },
                        event_subscriptions: Some(EventSubscription::ALL),
                        broadcast_capacity: 128,
                        connect_timeout: Duration::from_secs(10),
                        dangerous: None,
                    };
                    
                    match Client::connect_with_config(connect_opts).await {
                        Ok(client) => Ok(Arc::new(client)),
                        Err(e) => Err(AdapterError::connection_with_source(
                            format!("Failed to connect to OBS WebSocket: {}", e),
                            e
                        ))
                    }
                }
            }
        ).await;

        // Process connection result
        match connect_result {
            Ok(client) => {
                // Store the client
                *self.client.lock().await = Some(Arc::clone(&client));

                // Mark as connected using BaseAdapter
                self.base.set_connected(true);

                info!("Connected to OBS WebSocket server");
                
                // Record successful connection in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "connect_success",
                    Some(json!({
                        "host": config.host,
                        "port": config.port,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;

                // Publish initial state with retry
                let client_clone = Arc::clone(&client);
                let event_bus_clone = Arc::clone(&self.base.event_bus());
                let initial_state_result = crate::adapters::common::with_retry(
                    "publish_initial_state",
                    RetryOptions::default(),
                    move |attempt| {
                        let client_for_attempt = client_clone.clone();
                        let event_bus_for_attempt = event_bus_clone.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, "Retrying initial state publication");
                            }
                            match Self::publish_initial_state(&client_for_attempt, &event_bus_for_attempt).await {
                                Ok(_) => Ok(()),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "internal",
                                    format!("Failed to publish initial OBS state: {}", e),
                                    anyhow::anyhow!("{}", e)
                                ))
                            }
                        }
                    }
                ).await;
                
                // Process initial state result
                if let Err(e) = initial_state_result {
                    error!(error = %e, "Failed to publish initial OBS state after retries");
                } else {
                    debug!("Published initial OBS state");
                }

                // Create channel for shutdown signaling using BaseAdapter
                let (_, shutdown_rx) = self.base.create_shutdown_channel().await;

                // Start event handler
                let client_clone = Arc::clone(&client);
                let event_bus_clone = Arc::clone(&self.base.event_bus());
                let config_clone = config.clone();
                let callbacks_clone = Arc::clone(&self.callbacks);
                
                // Create a connected flag for the event handler
                let connected_flag = Arc::new(AtomicBool::new(true));

                let handle = tauri::async_runtime::spawn(
                    async move {
                        if let Err(e) = Self::handle_obs_events(
                            client_clone,
                            event_bus_clone,
                            config_clone,
                            connected_flag,
                            shutdown_rx,
                            callbacks_clone,
                        )
                        .await
                        {
                            error!(error = %e, "OBS event handler error");
                        }
                    }
                    .in_current_span(),
                );

                // Store the event handler using BaseAdapter
                self.base.set_event_handler(handle).await;

                // Set up retry for publishing connection event
                let event_bus_clone = Arc::clone(&self.base.event_bus());
                let payload = json!({
                    "host": config.host,
                    "port": config.port,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                let payload_clone = payload.clone();
                let publish_result = crate::adapters::common::with_retry(
                    "publish_connection_established",
                    RetryOptions::default(),
                    move |attempt| {
                        let event_bus_for_attempt = event_bus_clone.clone();
                        let payload_for_attempt = payload_clone.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, "Retrying connection event publication");
                            }
                            let stream_event = StreamEvent::new("obs", "connection.established", payload_for_attempt);
                            match event_bus_for_attempt.publish(stream_event).await {
                                Ok(receivers) => Ok(receivers),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "event",
                                    format!("Failed to publish connection event: {}", e),
                                    anyhow::anyhow!("{}", e)
                                ))
                            }
                        }
                    }
                ).await;
                
                // Process publish result
                match publish_result {
                    Ok(receivers) => {
                        debug!(receivers, "Published OBS connection event");
                    },
                    Err(e) => {
                        error!(error = %e, "Failed to publish OBS connection event after retries");
                    }
                }
                
                // Set up retry for triggering connection callbacks
                let callbacks_clone = self.callbacks.clone();
                let callback_event = ObsEvent::ConnectionChanged {
                    connected: true,
                    data: payload,
                };
                
                let callback_event_clone = callback_event.clone();
                let trigger_result = crate::adapters::common::with_retry(
                    "trigger_connection_callbacks",
                    RetryOptions::default(),
                    move |attempt| {
                        let callbacks_for_attempt = callbacks_clone.clone();
                        let event_for_attempt = callback_event_clone.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, "Retrying connection callback trigger");
                            }
                            match callbacks_for_attempt.trigger(event_for_attempt).await {
                                Ok(count) => Ok(count),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "event",
                                    format!("Failed to trigger connection callbacks: {}", e),
                                    anyhow::anyhow!("{}", e)
                                ))
                            }
                        }
                    }
                ).await;
                
                // Process trigger result
                match trigger_result {
                    Ok(count) => {
                        debug!(callbacks = count, "Triggered OBS connection callbacks");
                    },
                    Err(e) => {
                        error!(error = %e, "Failed to trigger OBS connection callbacks after retries");
                    }
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    host = %config.host,
                    port = config.port,
                    "Failed to connect to OBS WebSocket after retries"
                );
                
                // Record connection failure in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "connect_failure",
                    Some(json!({
                        "host": config.host,
                        "port": config.port,
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;

                // Set up retry for publishing failure event
                let event_bus_clone = Arc::clone(&self.base.event_bus());
                let error_clone = e.to_string();
                let config_clone = config.clone();
                
                let publish_result = crate::adapters::common::with_retry(
                    "publish_connection_failed",
                    RetryOptions::default(),
                    move |attempt| {
                        let event_bus_for_attempt = event_bus_clone.clone();
                        let error_for_attempt = error_clone.clone();
                        let config_for_attempt = config_clone.clone();
                        async move {
                            if attempt > 1 {
                                debug!(attempt, "Retrying connection failure event publication");
                            }
                            let payload = json!({
                                "host": config_for_attempt.host,
                                "port": config_for_attempt.port,
                                "error": error_for_attempt,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            let stream_event = StreamEvent::new("obs", "connection.failed", payload);
                            match event_bus_for_attempt.publish(stream_event).await {
                                Ok(receivers) => Ok(receivers),
                                Err(e) => Err(AdapterError::from_anyhow_error(
                                    "event",
                                    format!("Failed to publish connection failure event: {}", e),
                                    anyhow::anyhow!("{}", e)
                                ))
                            }
                        }
                    }
                ).await;
                
                if let Err(publish_err) = publish_result {
                    error!(
                        error = %publish_err,
                        "Failed to publish OBS connection failure event after retries"
                    );
                } else {
                    debug!("Published OBS connection failure event");
                }

                Err(e.into())
            }
        }
    }

    #[instrument(skip(self), fields(adapter = "obs"), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Record the disconnect operation in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "disconnect",
            Some(json!({
                "currently_connected": self.base.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        // Only disconnect if currently connected
        if !self.base.is_connected() {
            debug!("OBS adapter already disconnected");
            return Ok(());
        }

        info!("Disconnecting OBS adapter");

        // Set disconnected state using BaseAdapter
        self.base.set_connected(false);

        // Stop the event handler (send shutdown signal and abort the task)
        match self.base.stop_event_handler().await {
            Ok(_) => debug!("Successfully stopped OBS event handler"),
            Err(e) => warn!(error = %e, "Issues while stopping OBS event handler"),
        }

        // Clear the client
        *self.client.lock().await = None;
        debug!("Cleared OBS client reference");

        // Set up retry for publishing disconnection event
        let event_bus_clone = Arc::clone(&self.base.event_bus());
        let publish_result = crate::adapters::common::with_retry(
            "publish_disconnection_event",
            RetryOptions::default(),
            move |attempt| {
                let event_bus_for_attempt = event_bus_clone.clone();
                async move {
                    if attempt > 1 {
                        debug!(attempt, "Retrying disconnection event publication");
                    }
                    let payload = json!({
                        "message": "Disconnected from OBS WebSocket",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    
                    let stream_event = StreamEvent::new("obs", "connection.closed", payload);
                    match event_bus_for_attempt.publish(stream_event).await {
                        Ok(receivers) => Ok(receivers),
                        Err(e) => Err(AdapterError::from_anyhow_error(
                            "event",
                            format!("Failed to publish disconnection event: {}", e),
                            anyhow::anyhow!("{}", e)
                        ))
                    }
                }
            }
        ).await;
        
        // Process publish result
        match publish_result {
            Ok(receivers) => {
                debug!(receivers, "Published OBS disconnection event");
            },
            Err(e) => {
                error!(error = %e, "Failed to publish OBS disconnection event after retries");
            }
        }
        
        // Set up retry for triggering disconnection callbacks
        let callbacks_clone = self.callbacks.clone();
        let callback_event = ObsEvent::ConnectionChanged {
            connected: false,
            data: json!({
                "message": "Disconnected from OBS WebSocket",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        };
        
        let callback_event_clone = callback_event.clone();
        let trigger_result = crate::adapters::common::with_retry(
            "trigger_disconnection_callbacks",
            RetryOptions::default(),
            move |attempt| {
                let callbacks_for_attempt = callbacks_clone.clone();
                let event_for_attempt = callback_event_clone.clone();
                async move {
                    if attempt > 1 {
                        debug!(attempt, "Retrying disconnection callback trigger");
                    }
                    match callbacks_for_attempt.trigger(event_for_attempt).await {
                        Ok(count) => Ok(count),
                        Err(e) => Err(AdapterError::from_anyhow_error(
                            "event",
                            format!("Failed to trigger disconnection callbacks: {}", e),
                            anyhow::anyhow!("{}", e)
                        ))
                    }
                }
            }
        ).await;
        
        // Process trigger result
        match trigger_result {
            Ok(count) => {
                debug!(callbacks = count, "Triggered OBS disconnection callbacks");
            },
            Err(e) => {
                error!(error = %e, "Failed to trigger OBS disconnection callbacks after retries");
            }
        }
        
        // Record successful disconnection in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "disconnect_success",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;

        info!("Disconnected from OBS WebSocket");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }

    fn get_name(&self) -> &str {
        self.base.name()
    }

    #[instrument(skip(self, config), fields(adapter = "obs"), level = "debug")]
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        info!("Configuring OBS adapter");
        let current_config = self.config.read().await.clone();
        let mut new_config = current_config.clone();

        // Track if any connection-related settings changed
        let mut connection_settings_changed = false;

        // Update host if provided
        if let Some(host) = config.get("host").and_then(|v| v.as_str()) {
            if host != current_config.host {
                info!(old_host = %current_config.host, new_host = %host, "Updating OBS host");
                new_config.host = host.to_string();
                connection_settings_changed = true;
            }
        }

        // Update port if provided
        if let Some(port) = config.get("port").and_then(|v| v.as_u64()) {
            let port = port as u16;
            if port != current_config.port {
                info!(
                    old_port = current_config.port,
                    new_port = port,
                    "Updating OBS port"
                );
                new_config.port = port;
                connection_settings_changed = true;
            }
        }

        // Update password if provided
        if let Some(password) = config.get("password").and_then(|v| v.as_str()) {
            if password != current_config.password {
                debug!("Updating OBS password");
                new_config.password = password.to_string();
                connection_settings_changed = true;
            }
        }

        // Update auto_connect if provided
        if let Some(auto_connect) = config.get("auto_connect").and_then(|v| v.as_bool()) {
            if auto_connect != current_config.auto_connect {
                info!(
                    old_auto_connect = current_config.auto_connect,
                    new_auto_connect = auto_connect,
                    "Updating OBS auto_connect setting"
                );
                new_config.auto_connect = auto_connect;
            }
        }

        // Update include_scene_details if provided
        if let Some(include_details) = config
            .get("include_scene_details")
            .and_then(|v| v.as_bool())
        {
            if include_details != current_config.include_scene_details {
                info!(
                    old_value = current_config.include_scene_details,
                    new_value = include_details,
                    "Updating OBS include_scene_details setting"
                );
                new_config.include_scene_details = include_details;
            }
        }

        // Store the updated configuration
        *self.config.write().await = new_config.clone();
        debug!("Updated OBS adapter configuration");

        // If connection settings changed and we're connected, disconnect and reconnect
        if connection_settings_changed && self.base.is_connected() {
            info!(
                host = %new_config.host,
                port = new_config.port,
                "OBS connection settings changed, reconnecting"
            );

            self.disconnect().await?;

            // Wait a bit before reconnecting
            debug!(
                "Waiting {}ms before reconnecting to OBS",
                RECONNECT_DELAY_MS
            );
            sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;

            // Only reconnect if auto_connect is true
            if new_config.auto_connect {
                info!("Auto-reconnecting OBS adapter with new settings");
                self.connect().await?;
            } else {
                info!("Not auto-reconnecting OBS adapter (auto_connect is disabled)");
            }
        } else if connection_settings_changed {
            debug!("Connection settings changed but adapter not currently connected");
        } else {
            debug!("No connection-relevant settings changed");
        }

        Ok(())
    }
}

impl Clone for ObsAdapter {
    fn clone(&self) -> Self {
        // IMPORTANT: This is a proper implementation of Clone that ensures callback integrity.
        // When an adapter is cloned, it's essential that all shared state wrapped in
        // Arc is properly cloned with Arc::clone to maintain the same underlying instances.
        //
        // Common mistakes fixed here:
        // 1. Creating new RwLock/Mutex instances instead of sharing existing ones with Arc::clone
        // 2. Not properly sharing callback registries between clones
        // 3. Creating multiple copies of event buses, configs, or other shared state
        //
        // The correct pattern is to use Arc::clone for ALL fields that contain
        // state that should be shared between clones:
        // 1. BaseAdapter contains the event bus and connection state - use its clone implementation
        // 2. The client should be shared so all instances have the same connection
        // 3. The config should be shared so configuration changes affect all clones
        // 4. The callbacks registry must be shared so callbacks are maintained
        
        Self {
            base: self.base.clone(), // Use BaseAdapter's clone implementation
            client: Arc::clone(&self.client), // Share the same client
            config: Arc::clone(&self.config), // Share the same config
            callbacks: Arc::clone(&self.callbacks), // Share the same callback registry
        }
    }
}
