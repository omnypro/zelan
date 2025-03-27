// src/adapters/obs/adapter.rs
use crate::{
    adapters::{
        base::{BaseAdapter, ServiceHelperImpl},
        common::{AdapterError, TraceHelper},
    },
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use obws::events::OutputState;
use obws::{client::ConnectConfig, requests::EventSubscription, Client};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::async_runtime::{Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, instrument, trace, warn, Instrument};

// Import the callback registry for OBS events
use super::callback::{ObsCallbackRegistry, ObsEvent};

// Default connection settings
pub(crate) const DEFAULT_HOST: &str = "localhost";
pub(crate) const DEFAULT_PORT: u16 = 4455;
pub(crate) const DEFAULT_PASSWORD: &str = "";
pub(crate) const RECONNECT_DELAY_MS: u64 = 5000;
// SHUTDOWN_CHANNEL_SIZE not needed anymore since we use BaseAdapter

// OBS WebSocket connection configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObsConfig {
    /// OBS WebSocket host
    #[serde(default = "default_host")]
    pub host: String,
    /// OBS WebSocket port
    #[serde(default = "default_port")]
    pub port: u16,
    /// OBS WebSocket password
    #[serde(default = "default_password")]
    pub password: String,
    /// Auto-reconnect on disconnect
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
}

/// Default host
fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

/// Default port
fn default_port() -> u16 {
    DEFAULT_PORT
}

/// Default password
fn default_password() -> String {
    DEFAULT_PASSWORD.to_string()
}

/// Default auto-reconnect
fn default_auto_reconnect() -> bool {
    true
}

/// Default implementation for ObsConfig
impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            password: default_password(),
            auto_reconnect: default_auto_reconnect(),
        }
    }
}

/// Implementation of AdapterConfig for ObsConfig
impl crate::adapters::base::AdapterConfig for ObsConfig {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn from_json(json: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(json.clone())?)
    }

    fn adapter_type() -> &'static str {
        "obs"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.host.is_empty() {
            anyhow::bail!("OBS host cannot be empty");
        }
        if self.port == 0 {
            anyhow::bail!("OBS port cannot be 0");
        }
        Ok(())
    }
}

// The OBS adapter is now updated to work with obws v0.14.0:
// - ConnectConfig structure includes all required fields (event_subscriptions, broadcast_capacity, connect_timeout, dangerous)
// - Client::connect_with_config call now provides all required parameters
// - Event handling uses struct pattern matching (e.g., Event::CurrentProgramSceneChanged { id })
// - Using scenes().current_program_scene() API method instead of the deprecated scenes().current()

/// OBS adapter for interacting with OBS WebSocket
pub struct ObsAdapter {
    /// Base adapter
    base: BaseAdapter,
    /// OBS client
    obs_client: Mutex<Option<Client>>,
    /// OBS config
    config: RwLock<ObsConfig>,
    /// Shutdown signal sender
    shutdown_tx: RwLock<Option<mpsc::Sender<()>>>,
    /// Connected status
    connected: AtomicBool,
    /// Callback registry for OBS events - must be cloned correctly
    callback_registry: Arc<ObsCallbackRegistry>,
    /// Event subscription names
    event_subscriptions: Arc<Vec<String>>,
    /// Service helper for implementing ServiceAdapter
    service_helper: ServiceHelperImpl,
}

/// Implement Clone for OBS adapter
impl Clone for ObsAdapter {
    fn clone(&self) -> Self {
        Self {
            // Clone the base adapter (using the non-blocking implementation)
            base: self.base.clone(),

            // Each clone starts with no active OBS client - it will be initialized on connect
            obs_client: Mutex::new(None),

            // Clone config without blocking - use try_read
            config: RwLock::new(match self.config.try_read() {
                Ok(config) => config.clone(),
                // If we can't get the lock immediately, use default config
                Err(_) => ObsConfig::default(),
            }),

            // New clones have no shutdown channel until they connect
            shutdown_tx: RwLock::new(None),

            // Share connection state but create a new atomic with the same value
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),

            // CRITICAL: Share the callback registry using Arc::clone
            // This ensures all adapter clones trigger the same callbacks
            callback_registry: Arc::clone(&self.callback_registry),

            // Share immutable event subscriptions via Arc::clone
            event_subscriptions: Arc::clone(&self.event_subscriptions),

            // Share the service helper implementation
            service_helper: self.service_helper.clone(),
        }
    }
}

#[async_trait]
impl ServiceAdapter for ObsAdapter {
    /// Get the adapter type
    fn adapter_type(&self) -> &'static str {
        "obs"
    }

    /// Connect to OBS
    async fn connect(&self) -> Result<(), AdapterError> {
        trace!("OBS adapter connect() called");

        // Only connect if not already connected
        if self.connected.load(Ordering::SeqCst) {
            trace!("OBS adapter already connected, skipping connect");
            return Ok(());
        }

        // Record the connect operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "connect_start",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // Get the most up-to-date config
        let config = self.config.read().await.clone();

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Save host and port for error reporting
        let host = config.host.clone();
        let port = config.port;

        // Create connection config with all required fields
        let connect_config = ConnectConfig {
            host: config.host.clone(),
            port: config.port,
            password: if config.password.is_empty() {
                None
            } else {
                Some(config.password.clone())
            },
            event_subscriptions: Some(EventSubscription::ALL),
            broadcast_capacity: 128,
            connect_timeout: Duration::from_secs(10),
            dangerous: None,
        };
        debug!("Connecting to OBS: {}:{}", host, port);

        // Connect to OBS WebSocket using the configuration
        let client = match Client::connect_with_config(connect_config).await {
            Ok(client) => client,
            Err(e) => {
                // Recording connection failure in trace
                TraceHelper::record_adapter_operation(
                    self.adapter_type(),
                    "connect_failed",
                    Some(json!({
                        "error": e.to_string(),
                        "host": host,
                        "port": port,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })),
                )
                .await;

                error!("Failed to connect to OBS: {}", e);
                return Err(AdapterError::connection(format!(
                    "Failed to connect to OBS at {}:{}: {}",
                    host, port, e
                )));
            }
        };

        // Store client - Client doesn't implement Clone so we store it directly
        *self.obs_client.lock().await = Some(client);

        // Mark as connected
        self.connected.store(true, Ordering::SeqCst);

        // Create clones for event listener task
        let adapter_type = self.adapter_type().to_string();
        let event_bus_clone = self.base.event_bus();
        let callback_registry_clone = Arc::clone(&self.callback_registry);
        let connected_clone = AtomicBool::new(self.connected.load(Ordering::SeqCst));
        // Create a new client for the event listener task
        let obs_client = match Client::connect_with_config(ConnectConfig {
            host: config.host.clone(),
            port: config.port,
            password: if config.password.is_empty() {
                None
            } else {
                Some(config.password.clone())
            },
            event_subscriptions: Some(EventSubscription::ALL),
            broadcast_capacity: 128,
            connect_timeout: Duration::from_secs(10),
            dangerous: None,
        })
        .await
        {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create second client for event listener: {}", e);
                return Err(AdapterError::connection(format!(
                    "Failed to create second client for event listener: {}",
                    e
                )));
            }
        };
        let config_clone = config.clone();
        let subscriptions = Arc::clone(&self.event_subscriptions);

        // Start event listener task
        tokio::spawn(async move {
            // In v0.14.0, the events() method returns a stream directly
            // No need to call subscribe explicitly

            // Get the event stream
            let event_listener = match obs_client.events() {
                Ok(event_stream) => event_stream,
                Err(e) => {
                    error!("Failed to subscribe to OBS events: {}", e);
                    connected_clone.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // We already set up the event_listener above
            pin_mut!(event_listener);

            // Send connected event
            if let Err(e) = callback_registry_clone
                .trigger(ObsEvent::ConnectionChanged {
                    connected: true,
                    data: json!({
                        "host": config_clone.host,
                        "port": config_clone.port,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }),
                })
                .await
            {
                error!("Failed to trigger connection event: {}", e);
            }

            // Get current scene
            let initial_scene = match obs_client.scenes().current_program_scene().await {
                Ok(scene_name) => {
                    // Create scene data with available information
                    Some(serde_json::json!({
                        "name": scene_name
                    }))
                }
                Err(e) => {
                    warn!("Failed to get current scene: {}", e);
                    None
                }
            };

            // If we have an initial scene, trigger a scene changed event
            if let Some(scene) = initial_scene {
                if let Err(e) = callback_registry_clone
                    .trigger(ObsEvent::SceneChanged {
                        scene_name: scene["name"].as_str().unwrap_or("unknown").to_string(),
                        data: json!({
                            "initial": true,
                            "scene": scene,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }),
                    })
                    .await
                {
                    error!("Failed to trigger initial scene event: {}", e);
                }
            }

            // Get initial streaming status using current API
            let stream_status = match obs_client.streaming().status().await {
                Ok(status) => Some(status),
                Err(e) => {
                    warn!("Failed to get stream status: {}", e);
                    None
                }
            };

            // If we have stream status, trigger event
            if let Some(status) = stream_status {
                if let Err(e) = callback_registry_clone
                    .trigger(ObsEvent::StreamStateChanged {
                        streaming: status.active,
                        data: json!({
                            "initial": true,
                            "status": status,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }),
                    })
                    .await
                {
                    error!("Failed to trigger initial stream status event: {}", e);
                }
            }

            // Get initial recording status
            let recording_status = match obs_client.recording().status().await {
                Ok(status) => Some(status),
                Err(e) => {
                    warn!("Failed to get recording status: {}", e);
                    None
                }
            };

            // If we have recording status, trigger event
            if let Some(status) = recording_status {
                if let Err(e) = callback_registry_clone
                    .trigger(ObsEvent::RecordingStateChanged {
                        recording: status.active,
                        data: json!({
                            "initial": true,
                            "status": status,
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }),
                    })
                    .await
                {
                    error!("Failed to trigger initial recording status event: {}", e);
                }
            }

            // Event loop
            loop {
                tokio::select! {
                    // Shutdown signal
                    _ = shutdown_rx.recv() => {
                        info!("OBS adapter event listener shutting down");
                        break;
                    }
                    // Event from OBS
                    event = event_listener.next() => {
                        if let Some(event) = event {
                            // Log the event at trace level
                            trace!("Received OBS event: {:?}", event);

                            // Map OBS events to our event model
                            let adapter_type_str = adapter_type.clone();
                            let event_bus = event_bus_clone.clone();

                            // Handle specific event types
                            match event {
                                obws::events::Event::CurrentProgramSceneChanged { id } => {
                                    // Convert the scene ID to a string using debug formatting since Display isn't implemented
                                    let scene_name_str = format!("{:?}", id);

                                    // Create JSON for event bus
                                    let event_data = json!({
                                        "scene_name": scene_name_str
                                    });

                                    // Convert to JSON for event bus
                                    if let Ok(data) = serde_json::to_value(&event_data) {
                                        // Publish to event bus
                                        let _ = event_bus.publish(StreamEvent::new(
                                            &adapter_type_str,
                                            "scene.changed",
                                            data.clone(),
                                        )).await;

                                        // Trigger callback
                                        if let Err(e) = callback_registry_clone.trigger(
                                            ObsEvent::SceneChanged {
                                                scene_name: scene_name_str,
                                                data: json!({
                                                    "scene": data,
                                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                                }),
                                            }
                                        ).await {
                                            error!("Failed to trigger scene changed event: {}", e);
                                        }
                                    }
                                }
                                obws::events::Event::StreamStateChanged { active, state } => {
                                    // Handle stream state change
                                    let stream_data = json!({
                                        "active": active,
                                        "state": format!("{:?}", state)
                                    });

                                    if let Ok(data) = serde_json::to_value(&stream_data) {
                                        // Determine the exact event type based on state
                                        let (event_type, is_streaming) = match state {
                                            OutputState::Starting => ("stream.starting", false),
                                            OutputState::Started => ("stream.started", true),
                                            OutputState::Stopping => ("stream.stopping", true),
                                            OutputState::Stopped => ("stream.stopped", false),
                                            _ => {
                                                warn!("Unknown stream state");
                                                ("stream.unknown", false)
                                            }
                                        };

                                        // Publish to event bus
                                        let _ = event_bus.publish(StreamEvent::new(
                                            &adapter_type_str,
                                            event_type,
                                            data.clone(),
                                        )).await;

                                        // Trigger callback
                                        if let Err(e) = callback_registry_clone.trigger(
                                            ObsEvent::StreamStateChanged {
                                                streaming: is_streaming,
                                                data: json!({
                                                    "state": format!("{:?}", state),
                                                    "full_data": data,
                                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                                }),
                                            }
                                        ).await {
                                            error!("Failed to trigger stream state event: {}", e);
                                        }
                                    }
                                }
                                obws::events::Event::RecordStateChanged { active, state, path } => {
                                    // Handle recording state change
                                    let record_data = json!({
                                        "active": active,
                                        "state": format!("{:?}", state),
                                        "path": path
                                    });

                                    if let Ok(data) = serde_json::to_value(&record_data) {
                                        // Determine the exact event type based on state
                                        let (event_type, is_recording) = match state {
                                            OutputState::Starting => ("recording.starting", false),
                                            OutputState::Started => ("recording.started", true),
                                            OutputState::Stopping => ("recording.stopping", true),
                                            OutputState::Stopped => ("recording.stopped", false),
                                            _ => {
                                                warn!("Unknown recording state");
                                                ("recording.unknown", false)
                                            }
                                        };

                                        // Publish to event bus
                                        let _ = event_bus.publish(StreamEvent::new(
                                            &adapter_type_str,
                                            event_type,
                                            data.clone(),
                                        )).await;

                                        // Trigger callback
                                        if let Err(e) = callback_registry_clone.trigger(
                                            ObsEvent::RecordingStateChanged {
                                                recording: is_recording,
                                                data: json!({
                                                    "state": format!("{:?}", state),
                                                    "full_data": data,
                                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                                }),
                                            }
                                        ).await {
                                            error!("Failed to trigger recording state event: {}", e);
                                        }
                                    }
                                }
                                // For all other events, publish them as-is
                                _ => {
                                    // Create a generic event name from the event variant
                                    let event_name = format!("obs.{}", match event {
                                        obws::events::Event::InputCreated { .. } => "input.created",
                                        obws::events::Event::InputRemoved { .. } => "input.removed",
                                        obws::events::Event::InputNameChanged { .. } => "input.nameChanged",
                                        // Add other event types as needed
                                        _ => "generic",
                                    });

                                    // Convert to JSON for event bus
                                    if let Ok(data) = serde_json::to_value(&event) {
                                        // Publish to event bus if it's in our subscribed events
                                        if subscriptions.is_empty() ||
                                           subscriptions.contains(&event_name) ||
                                           subscriptions.contains(&format!("obs.*")) {
                                            let _ = event_bus.publish(StreamEvent::new(
                                                &adapter_type_str,
                                                &event_name,
                                                data,
                                            )).await;
                                        }
                                    }
                                }
                            }
                        } else {
                            // If we get None, the stream has ended
                            warn!("OBS event stream ended");
                            connected_clone.store(false, Ordering::SeqCst);

                            // Trigger disconnected event
                            if let Err(e) = callback_registry_clone.trigger(
                                ObsEvent::ConnectionChanged {
                                    connected: false,
                                    data: json!({
                                        "error": "Event stream ended",
                                        "timestamp": chrono::Utc::now().to_rfc3339()
                                    }),
                                }
                            ).await {
                                error!("Failed to trigger disconnection event: {}", e);
                            }

                            break;
                        }
                    }
                }
            }

            // Final disconnection logic
            connected_clone.store(false, Ordering::SeqCst);

            // If auto-reconnect is enabled, spawn a reconnect task
            if config_clone.auto_reconnect {
                // Use a clone for the reconnect task
                let adapter_type = adapter_type.clone();
                // Save host and port for error reporting and events
                let host = config_clone.host.clone();
                let port = config_clone.port;

                // Create a new connect config with the same values
                let reconnect_config = ConnectConfig {
                    host: config_clone.host.clone(),
                    port: config_clone.port,
                    password: if config_clone.password.is_empty() {
                        None
                    } else {
                        Some(config_clone.password.clone())
                    },
                    event_subscriptions: Some(EventSubscription::ALL),
                    broadcast_capacity: 128,
                    connect_timeout: Duration::from_secs(10),
                    dangerous: None,
                };
                let callback_registry = callback_registry_clone.clone();

                tokio::spawn(async move {
                    info!(
                        "Attempting to reconnect to OBS after {}ms",
                        RECONNECT_DELAY_MS
                    );

                    // Wait before reconnecting
                    sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;

                    // Attempt to reconnect
                    match Client::connect_with_config(reconnect_config).await {
                        Ok(_) => {
                            info!("Successfully reconnected to OBS");
                            connected_clone.store(true, Ordering::SeqCst);

                            // Trigger connected event
                            if let Err(e) = callback_registry
                                .trigger(ObsEvent::ConnectionChanged {
                                    connected: true,
                                    data: json!({
                                        "host": host,
                                        "port": port,
                                        "reconnected": true,
                                        "timestamp": chrono::Utc::now().to_rfc3339()
                                    }),
                                })
                                .await
                            {
                                error!("Failed to trigger reconnection event: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to reconnect to OBS: {}", e);

                            // Record reconnect failure
                            TraceHelper::record_adapter_operation(
                                &adapter_type,
                                "reconnect_failed",
                                Some(json!({
                                    "error": e.to_string(),
                                    "host": host,
                                    "port": port,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                })),
                            )
                            .await;
                        }
                    }
                });
            }
        });

        // Record successful connection
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "connect_success",
            Some(json!({
                "host": host,
                "port": port,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        Ok(())
    }

    /// Disconnect from OBS
    async fn disconnect(&self) -> Result<(), AdapterError> {
        trace!("OBS adapter disconnect() called");

        // Only disconnect if connected
        if !self.connected.load(Ordering::SeqCst) {
            trace!("OBS adapter not connected, skipping disconnect");
            return Ok(());
        }

        // Record the disconnect operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "disconnect",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // Send shutdown signal to event listener
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            // Ignore send errors since the receiver might be gone already
            let _ = tx.send(()).await;
        }

        // Clear client
        *self.obs_client.lock().await = None;

        // Mark as disconnected
        self.connected.store(false, Ordering::SeqCst);

        // Trigger connection changed event
        if let Err(e) = self
            .callback_registry
            .trigger(ObsEvent::ConnectionChanged {
                connected: false,
                data: json!({
                    "manual_disconnect": true,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }),
            })
            .await
        {
            warn!("Failed to trigger disconnect event: {}", e);
        }

        // Record successful disconnect
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "disconnect_success",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        Ok(())
    }

    /// Check the connection status
    async fn check_connection(&self) -> Result<bool, AdapterError> {
        Ok(self.connected.load(Ordering::SeqCst))
    }

    /// Get the adapter status as JSON
    async fn get_status(&self) -> Result<serde_json::Value, AdapterError> {
        // Create base status
        let mut status = json!({
            "adapter_type": self.adapter_type(),
            "name": self.base.name(),
            "id": self.base.id(),
            "connected": self.connected.load(Ordering::SeqCst),
        });

        // Add config (without sensitive info)
        let config = self.config.read().await;
        status["config"] = json!({
            "host": config.host,
            "port": config.port,
            "has_password": !config.password.is_empty(),
            "auto_reconnect": config.auto_reconnect,
        });

        // If connected, try to get some OBS information
        if self.connected.load(Ordering::SeqCst) {
            if let Some(client) = self.obs_client.lock().await.as_ref() {
                // Get current scene
                if let Ok(scene_name) = client.scenes().current_program_scene().await {
                    status["current_scene"] = json!({
                        "name": scene_name,
                    });
                }

                // Get stream status
                if let Ok(stream_status) = client.streaming().status().await {
                    status["streaming"] = json!({
                        "active": stream_status.active,
                    });
                }

                // Get recording status
                if let Ok(recording_status) = client.recording().status().await {
                    status["recording"] = json!({
                        "active": recording_status.active,
                    });
                }
            }
        }

        Ok(status)
    }

    /// Handle lifecycle events from the manager
    async fn handle_lifecycle_event(
        &self,
        event: &str,
        _data: Option<&serde_json::Value>,
    ) -> Result<(), AdapterError> {
        debug!("Handling lifecycle event: {}", event);

        match event {
            "initialize" => {
                // Already handled in the constructor
                Ok(())
            }
            _ => {
                // Ignore unknown events
                Ok(())
            }
        }
    }

    /// Execute a command
    async fn execute_command(
        &self,
        command: &str,
        args: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, AdapterError> {
        // Check if connected for commands that require a connection
        let requires_connection = match command {
            "status" | "connect" | "disconnect" => false,
            _ => true,
        };

        if requires_connection && !self.connected.load(Ordering::SeqCst) {
            return Err(AdapterError::connection("Not connected to OBS"));
        }

        // Execute the command
        match command {
            "status" => self.get_status().await,
            "connect" => {
                self.connect().await?;
                Ok(json!({"status": "connected"}))
            }
            "disconnect" => {
                self.disconnect().await?;
                Ok(json!({"status": "disconnected"}))
            }
            "get_scenes" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    let scenes = client.scenes().list().await.map_err(|e| {
                        AdapterError::api(format!("Failed to get OBS scenes: {}", e))
                    })?;

                    Ok(serde_json::to_value(scenes).unwrap_or_default())
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "switch_scene" => {
                if let Some(args) = args {
                    if let Some(scene_name) = args.get("scene_name") {
                        if let Some(scene_name) = scene_name.as_str() {
                            if let Some(client) = self.obs_client.lock().await.as_ref() {
                                client
                                    .scenes()
                                    .set_current_program_scene(scene_name)
                                    .await
                                    .map_err(|e| {
                                        AdapterError::api(format!(
                                            "Failed to switch to scene '{}': {}",
                                            scene_name, e
                                        ))
                                    })?;

                                Ok(json!({"status": "success", "scene": scene_name}))
                            } else {
                                Err(AdapterError::internal("No OBS client available"))
                            }
                        } else {
                            Err(AdapterError::config(
                                "Invalid argument: scene_name must be a string",
                            ))
                        }
                    } else {
                        Err(AdapterError::config(
                            "Missing required argument: scene_name",
                        ))
                    }
                } else {
                    Err(AdapterError::config("Missing required arguments"))
                }
            }
            "start_streaming" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    client.streaming().start().await.map_err(|e| {
                        AdapterError::api(format!("Failed to start streaming: {}", e))
                    })?;

                    Ok(json!({"status": "streaming_started"}))
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "stop_streaming" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    client.streaming().stop().await.map_err(|e| {
                        AdapterError::api(format!("Failed to stop streaming: {}", e))
                    })?;

                    Ok(json!({"status": "streaming_stopped"}))
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "start_recording" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    client.recording().start().await.map_err(|e| {
                        AdapterError::api(format!("Failed to start recording: {}", e))
                    })?;

                    Ok(json!({"status": "recording_started"}))
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "stop_recording" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    client.recording().stop().await.map_err(|e| {
                        AdapterError::api(format!("Failed to stop recording: {}", e))
                    })?;

                    Ok(json!({"status": "recording_stopped"}))
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "get_streaming_status" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    let status = client.streaming().status().await.map_err(|e| {
                        AdapterError::api(format!("Failed to get streaming status: {}", e))
                    })?;

                    Ok(serde_json::to_value(status).unwrap_or_default())
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            "get_recording_status" => {
                if let Some(client) = self.obs_client.lock().await.as_ref() {
                    let status = client.recording().status().await.map_err(|e| {
                        AdapterError::api(format!("Failed to get recording status: {}", e))
                    })?;

                    Ok(serde_json::to_value(status).unwrap_or_default())
                } else {
                    Err(AdapterError::internal("No OBS client available"))
                }
            }
            _ => Err(AdapterError::invalid_command(format!(
                "Unknown command: {}",
                command
            ))),
        }
    }

    /// Get a feature by key from service helper
    async fn get_feature(&self, key: &str) -> Result<Option<String>, AdapterError> {
        self.service_helper.get_feature(key).await
    }
}

/// Implementation for OBS adapter
impl ObsAdapter {
    /// Create a new OBS adapter
    pub fn new(
        name: &str,
        id: &str,
        event_bus: Arc<EventBus>,
        config: Option<ObsConfig>,
        event_subscriptions: Option<Vec<String>>,
    ) -> Self {
        // Create base adapter
        let config = config.unwrap_or_else(|| ObsConfig {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            password: DEFAULT_PASSWORD.to_string(),
            auto_reconnect: true,
        });

        // Create callback registry
        let callback_registry = Arc::new(ObsCallbackRegistry::new());

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

        // Create adapter
        Self {
            base,
            obs_client: Mutex::new(None),
            config: RwLock::new(config),
            shutdown_tx: RwLock::new(None),
            connected: AtomicBool::new(false),
            callback_registry,
            event_subscriptions: Arc::new(event_subscriptions.unwrap_or_default()),
            service_helper,
        }
    }

    /// Register a callback for OBS events
    pub async fn register_callback<F>(
        &self,
        callback: F,
    ) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(ObsEvent) -> Result<()> + Send + Sync + 'static,
    {
        Ok(self.callback_registry.register(callback).await)
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

    /// Create a new adapter with the provided configuration
    pub fn with_config(name: &str, id: &str, event_bus: Arc<EventBus>, config: ObsConfig) -> Self {
        Self::new(name, id, event_bus, Some(config), None)
    }
}

#[cfg(test)]
mod tests {
    pub use crate::adapters::tests::obs_test::*;
}
