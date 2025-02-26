// src/adapters/obs.rs
use crate::{EventBus, ServiceAdapter, StreamEvent};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use obws::{client::ConnectConfig, requests::EventSubscription, Client};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::async_runtime;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, instrument, warn, Instrument};

// Default connection settings
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 4455;
const DEFAULT_PASSWORD: &str = "";
const RECONNECT_DELAY_MS: u64 = 5000;
const SHUTDOWN_CHANNEL_SIZE: usize = 1;

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

/// OBS Studio adapter for connecting to OBS via its WebSocket API
pub struct ObsAdapter {
    name: String,
    event_bus: Arc<EventBus>,
    connected: AtomicBool,
    client: Mutex<Option<Arc<Client>>>,
    config: RwLock<ObsConfig>,
    event_handler: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    shutdown_signal: Mutex<Option<mpsc::Sender<()>>>,
}

impl ObsAdapter {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            name: "obs".to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            client: Mutex::new(None),
            config: RwLock::new(ObsConfig::default()),
            event_handler: Mutex::new(None),
            shutdown_signal: Mutex::new(None),
        }
    }

    /// Creates a new OBS adapter with the given configuration
    pub fn with_config(event_bus: Arc<EventBus>, config: ObsConfig) -> Self {
        Self {
            name: "obs".to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            client: Mutex::new(None),
            config: RwLock::new(config),
            event_handler: Mutex::new(None),
            shutdown_signal: Mutex::new(None),
        }
    }

    /// Handles incoming OBS events and converts them to StreamEvents
    #[instrument(
        skip(client, event_bus, config, connected, shutdown_rx),
        level = "debug"
    )]
    async fn handle_obs_events(
        client: Arc<Client>,
        event_bus: Arc<EventBus>,
        config: ObsConfig,
        connected: Arc<AtomicBool>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        // Set up OBS event listener
        let events = match client.events() {
            Ok(events) => events,
            Err(e) => {
                error!(error = %e, "Failed to create OBS event stream");
                return Err(anyhow!("Failed to create OBS event stream: {}", e));
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
                    break;
                }

                // Check connection status periodically
                _ = sleep(Duration::from_secs(5)) => {
                    if !connected.load(Ordering::SeqCst) {
                        info!("Connection flag set to false, stopping OBS event handler");
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

                            let mut payload = json!({
                                "scene_name": scene_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            // If configured to include scene details, gather them
                            if config.include_scene_details {
                                debug!("Fetching additional scene details");
                                match client.scenes().list().await {
                                    Ok(scene_list) => {
                                        // Find the current scene in the list
                                        if let Some(scene) =
                                            scene_list.scenes.iter().find(|s| s.id.name == id.name)
                                        {
                                            // Add additional scene information
                                            payload["scene_index"] = json!(scene.index);
                                            debug!(
                                                scene_index = scene.index,
                                                "Added scene details"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "Failed to fetch scene details");
                                    }
                                }
                            }

                            // Create and publish the event
                            let stream_event = StreamEvent::new("obs", "scene.changed", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS scene change event");
                            } else {
                                debug!(scene = %scene_name, "Published scene change event");
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

                            let stream_event = StreamEvent::new("obs", event_type, payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS stream state event");
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
        // Check if already connected
        if self.connected.load(Ordering::SeqCst) {
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

        // Build the connection options - using password only if provided
        let connect_opts = ConnectConfig {
            host: config.host.clone(),
            port: config.port,
            password: if config.password.is_empty() {
                debug!("No password provided for OBS connection");
                None
            } else {
                debug!("Using password for OBS connection");
                Some(config.password.clone())
            },
            event_subscriptions: Some(EventSubscription::ALL),
            broadcast_capacity: 128,
            connect_timeout: Duration::from_secs(10),
            dangerous: None,
        };

        // Log the actual connection parameters to confirm they're being used
        debug!(
            host = %connect_opts.host,
            port = connect_opts.port,
            "Using OBS connection parameters"
        );

        // Attempt to connect
        match Client::connect_with_config(connect_opts).await {
            Ok(client) => {
                // Create Arc-wrapped client
                let client = Arc::new(client);

                // Store the client
                *self.client.lock().await = Some(Arc::clone(&client));

                // Mark as connected
                self.connected.store(true, Ordering::SeqCst);

                info!("Connected to OBS WebSocket server");

                // Publish initial state
                match Self::publish_initial_state(&client, &self.event_bus).await {
                    Ok(_) => debug!("Published initial OBS state"),
                    Err(e) => error!(error = %e, "Failed to publish initial OBS state"),
                }

                // Create channel for shutdown signaling
                let (shutdown_tx, shutdown_rx) = mpsc::channel(SHUTDOWN_CHANNEL_SIZE);
                *self.shutdown_signal.lock().await = Some(shutdown_tx);

                // Create a separate Arc for the connected flag
                let connected_flag = Arc::new(AtomicBool::new(true));

                // Start event handler with proper Arc references
                let client_clone = Arc::clone(&client);
                let event_bus_clone = Arc::clone(&self.event_bus);
                let config_clone = config.clone();

                let handle = tauri::async_runtime::spawn(
                    async move {
                        if let Err(e) = Self::handle_obs_events(
                            client_clone,
                            event_bus_clone,
                            config_clone,
                            connected_flag,
                            shutdown_rx,
                        )
                        .await
                        {
                            error!(error = %e, "OBS event handler error");
                        }
                    }
                    .in_current_span(),
                );

                // Store the event handler
                *self.event_handler.lock().await = Some(handle);

                // Publish connection success event
                let payload = json!({
                    "host": config.host,
                    "port": config.port,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "connection.established", payload);
                if let Err(e) = self.event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish OBS connection event");
                } else {
                    debug!("Published OBS connection event");
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to OBS WebSocket: {}", e);
                error!(
                    error = %e,
                    host = %config.host,
                    port = config.port,
                    "Failed to connect to OBS WebSocket"
                );

                // Publish connection failed event
                let payload = json!({
                    "host": config.host,
                    "port": config.port,
                    "error": error_msg.clone(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "connection.failed", payload);
                if let Err(publish_err) = self.event_bus.publish(stream_event).await {
                    error!(
                        error = %publish_err,
                        "Failed to publish OBS connection failure event"
                    );
                } else {
                    debug!("Published OBS connection failure event");
                }

                Err(anyhow!(error_msg))
            }
        }
    }

    #[instrument(skip(self), fields(adapter = "obs"), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if currently connected
        if !self.connected.load(Ordering::SeqCst) {
            debug!("OBS adapter already disconnected");
            return Ok(());
        }

        info!("Disconnecting OBS adapter");

        // Set disconnected state to stop event handling
        self.connected.store(false, Ordering::SeqCst);

        // Send shutdown signal if exists
        if let Some(shutdown_sender) = self.shutdown_signal.lock().await.take() {
            debug!("Sending shutdown signal to OBS event handler");
            if let Err(e) = shutdown_sender.send(()).await {
                warn!(error = %e, "Failed to send shutdown signal to OBS event handler");
            }
            // Small delay to allow shutdown signal to be processed
            sleep(Duration::from_millis(100)).await;
        } else {
            debug!("No shutdown signal channel available");
        }

        // Cancel the event handler task if it's still running
        if let Some(handle) = self.event_handler.lock().await.take() {
            // Simplest solution - just abort the task
            debug!("Aborting OBS event handler task");
            handle.abort();
        } else {
            debug!("No event handler task to abort");
        }

        // Clear the client
        *self.client.lock().await = None;
        debug!("Cleared OBS client reference");

        // Publish disconnection event
        let payload = json!({
            "message": "Disconnected from OBS WebSocket",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let stream_event = StreamEvent::new("obs", "connection.closed", payload);
        if let Err(e) = self.event_bus.publish(stream_event).await {
            error!(error = %e, "Failed to publish OBS disconnection event");
        } else {
            debug!("Published OBS disconnection event");
        }

        info!("Disconnected from OBS WebSocket");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn get_name(&self) -> &str {
        &self.name
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
        if connection_settings_changed && self.connected.load(Ordering::SeqCst) {
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
        Self {
            name: self.name.clone(),
            event_bus: Arc::clone(&self.event_bus),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            client: Mutex::new(None), // Don't clone the client itself
            config: RwLock::new(self.config.blocking_read().clone()),
            event_handler: Mutex::new(None), // Don't clone the task handle
            shutdown_signal: Mutex::new(None), // Don't clone the shutdown signal
        }
    }
}
