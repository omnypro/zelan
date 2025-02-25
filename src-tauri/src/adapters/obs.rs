// src/adapters/obs.rs
use crate::{EventBus, ServiceAdapter, StreamEvent};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use obws::{client::ConnectConfig, requests::EventSubscription, Client};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{sleep, Duration};

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
    event_handler: Mutex<Option<tokio::task::JoinHandle<()>>>,
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
    async fn handle_obs_events(
        client: Arc<Client>,
        event_bus: Arc<EventBus>,
        config: ObsConfig,
        connected: Arc<AtomicBool>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        // Set up OBS event listener
        let events = client.events()?;
        pin_mut!(events);

        println!("OBS event listener started");

        // Continue as long as connected is true and no shutdown signal is received
        loop {
            // Listen for the next event with a timeout to allow for checking connected state
            let maybe_event = tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    println!("Received shutdown signal for OBS event handler");
                    break;
                }

                // Check connection status periodically
                _ = sleep(Duration::from_secs(5)) => {
                    if !connected.load(Ordering::SeqCst) {
                        println!("Connection flag set to false, stopping OBS event handler");
                        break;
                    }
                    continue;
                }

                // Try to get the next event
                ev = events.next() => ev,
            };

            if !connected.load(Ordering::SeqCst) {
                println!("Connection flag set to false, stopping OBS event handler");
                break;
            }

            // Handle next event from the stream
            match maybe_event {
                Some(event) => {
                    // Convert OBS event to our internal format
                    match event {
                        obws::events::Event::CurrentProgramSceneChanged { id } => {
                            let mut payload = json!({
                                "scene_name": id.name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            // If configured to include scene details, gather them
                            if config.include_scene_details {
                                if let Ok(scene_list) = client.scenes().list().await {
                                    // Find the current scene in the list
                                    if let Some(scene) =
                                        scene_list.scenes.iter().find(|s| s.id.name == id.name)
                                    {
                                        // Add additional scene information
                                        payload["scene_index"] = json!(scene.index);
                                    }
                                }
                            }

                            // Create and publish the event
                            let stream_event = StreamEvent::new("obs", "scene.changed", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish OBS scene change event: {}", e);
                            }
                        }
                        obws::events::Event::SceneItemEnableStateChanged {
                            scene,
                            item_id,
                            enabled,
                            ..
                        } => {
                            let payload = json!({
                                "scene_name": scene,
                                "scene_item_id": item_id,
                                "enabled": enabled,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "scene_item.visibility_changed", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish OBS scene item event: {}", e);
                            }
                        }
                        obws::events::Event::StreamStateChanged { active, .. } => {
                            let event_type = if active {
                                "stream.started"
                            } else {
                                "stream.stopped"
                            };

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish OBS stream state event: {}", e);
                            }
                        }
                        obws::events::Event::RecordStateChanged { active, .. } => {
                            let event_type = if active {
                                "recording.started"
                            } else {
                                "recording.stopped"
                            };

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish OBS recording state event: {}", e);
                            }
                        }
                        // Provide connection events
                        obws::events::Event::ExitStarted => {
                            let payload = json!({
                                "message": "OBS is shutting down",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "connection.closing", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish OBS exit event: {}", e);
                            }

                            // OBS is shutting down, so we should stop
                            connected.store(false, Ordering::SeqCst);
                            break;
                        }
                        // Catch all other events and log them with generic handling
                        _ => {
                            // We can add specific handling for more event types as needed
                            let event_name = format!("{:?}", event);

                            // Create a simple payload with event details
                            let payload = json!({
                                "event_type": event_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", "event.generic", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                eprintln!("Failed to publish generic OBS event: {}", e);
                            }
                        }
                    }
                }
                // In obws 0.14 the events stream returns Event directly, not Result<Event, Error>
                // This branch is kept for future compatibility or in case errors are added
                // None => {
                None => {
                    // End of events stream usually means connection closed
                    println!("OBS events stream ended");

                    // If we're still supposed to be connected, this is unexpected
                    if connected.load(Ordering::SeqCst) {
                        // Send a disconnection event
                        let payload = json!({
                            "message": "OBS WebSocket connection closed unexpectedly",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        let stream_event = StreamEvent::new("obs", "connection.closed", payload);
                        if let Err(e) = event_bus.publish(stream_event).await {
                            eprintln!("Failed to publish OBS disconnection event: {}", e);
                        }

                        // We're no longer connected
                        connected.store(false, Ordering::SeqCst);
                    }

                    break;
                }
            }
        }

        println!("OBS event handler stopped");
        Ok(())
    }

    /// Get current scenes and publish an initial state
    async fn publish_initial_state(client: &Arc<Client>, event_bus: &Arc<EventBus>) -> Result<()> {
        println!("Publishing initial OBS state...");

        // Get version information
        if let Ok(version) = client.general().version().await {
            let payload = json!({
                "obs_version": version.obs_version,
                "websocket_version": version.obs_web_socket_version,
                "platform": version.platform,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            let stream_event = StreamEvent::new("obs", "system.info", payload);
            if let Err(e) = event_bus.publish(stream_event).await {
                eprintln!("Failed to publish OBS version info: {}", e);
            }
        }

        // Get current scene
        if let Ok(current_scene) = client.scenes().current_program_scene().await {
            let payload = json!({
                "scene_name": current_scene,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "initial": true,
            });

            let stream_event = StreamEvent::new("obs", "scene.current", payload);
            if let Err(e) = event_bus.publish(stream_event).await {
                eprintln!("Failed to publish current scene: {}", e);
            }
        }

        // Get all scenes
        if let Ok(scenes) = client.scenes().list().await {
            let payload = json!({
                "scenes": scenes.scenes,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            let stream_event = StreamEvent::new("obs", "scenes.list", payload);
            if let Err(e) = event_bus.publish(stream_event).await {
                eprintln!("Failed to publish scenes list: {}", e);
            }
        }

        // Get stream status
        if let Ok(status) = client.streaming().status().await {
            let payload = json!({
                "active": status.active,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "initial": true,
            });

            let stream_event = StreamEvent::new("obs", "stream.status", payload);
            if let Err(e) = event_bus.publish(stream_event).await {
                eprintln!("Failed to publish stream status: {}", e);
            }
        }

        // Get recording status
        if let Ok(status) = client.recording().status().await {
            let payload = json!({
                "active": status.active,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "initial": true,
            });

            let stream_event = StreamEvent::new("obs", "recording.status", payload);
            if let Err(e) = event_bus.publish(stream_event).await {
                eprintln!("Failed to publish recording status: {}", e);
            }
        }

        println!("Initial OBS state published");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for ObsAdapter {
    async fn connect(&self) -> Result<()> {
        // Check if already connected
        if self.connected.load(Ordering::SeqCst) {
            println!("OBS adapter is already connected");
            return Ok(());
        }

        // Get connection configuration
        let config = self.config.read().await.clone();

        // Connect to OBS WebSocket server
        println!(
            "Connecting to OBS WebSocket at {}:{}",
            config.host, config.port
        );

        // Build the connection options - using password only if provided
        let connect_opts = ConnectConfig {
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

        // Log the actual connection parameters to confirm they're being used
        println!(
            "Using connection parameters - host: {}, port: {}",
            connect_opts.host, connect_opts.port
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

                println!("Connected to OBS WebSocket server");

                // Publish initial state
                if let Err(e) = Self::publish_initial_state(&client, &self.event_bus).await {
                    eprintln!("Failed to publish initial OBS state: {}", e);
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
                let handle = tokio::spawn(async move {
                    if let Err(e) = Self::handle_obs_events(
                        client_clone,
                        event_bus_clone,
                        config_clone,
                        connected_flag,
                        shutdown_rx,
                    )
                    .await
                    {
                        eprintln!("OBS event handler error: {}", e);
                    }
                });

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
                    eprintln!("Failed to publish OBS connection event: {}", e);
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to OBS WebSocket: {}", e);
                eprintln!("{}", error_msg);

                // Publish connection failed event
                let payload = json!({
                    "host": config.host,
                    "port": config.port,
                    "error": error_msg,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "connection.failed", payload);
                if let Err(publish_err) = self.event_bus.publish(stream_event).await {
                    eprintln!(
                        "Failed to publish OBS connection failure event: {}",
                        publish_err
                    );
                }

                Err(anyhow!(error_msg))
            }
        }
    }

    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if currently connected
        if !self.connected.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Set disconnected state to stop event handling
        self.connected.store(false, Ordering::SeqCst);

        // Send shutdown signal if exists
        if let Some(shutdown_sender) = self.shutdown_signal.lock().await.take() {
            let _ = shutdown_sender.send(()).await;
            // Small delay to allow shutdown signal to be processed
            sleep(Duration::from_millis(100)).await;
        }

        // Cancel the event handler task if it's still running
        if let Some(handle) = self.event_handler.lock().await.take() {
            // Simplest solution - just abort the task
            println!("Aborting OBS event handler");
            handle.abort();
        }

        // Clear the client
        *self.client.lock().await = None;

        // Publish disconnection event
        let payload = json!({
            "message": "Disconnected from OBS WebSocket",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let stream_event = StreamEvent::new("obs", "connection.closed", payload);
        if let Err(e) = self.event_bus.publish(stream_event).await {
            eprintln!("Failed to publish OBS disconnection event: {}", e);
        }

        println!("Disconnected from OBS WebSocket");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        let current_config = self.config.read().await.clone();
        let mut new_config = current_config.clone();

        // Update host if provided
        if let Some(host) = config.get("host").and_then(|v| v.as_str()) {
            new_config.host = host.to_string();
        }

        // Update port if provided
        if let Some(port) = config.get("port").and_then(|v| v.as_u64()) {
            new_config.port = port as u16;
        }

        // Update password if provided
        if let Some(password) = config.get("password").and_then(|v| v.as_str()) {
            new_config.password = password.to_string();
        }

        // Update auto_connect if provided
        if let Some(auto_connect) = config.get("auto_connect").and_then(|v| v.as_bool()) {
            new_config.auto_connect = auto_connect;
        }

        // Update include_scene_details if provided
        if let Some(include_details) = config
            .get("include_scene_details")
            .and_then(|v| v.as_bool())
        {
            new_config.include_scene_details = include_details;
        }

        // Store the updated configuration
        *self.config.write().await = new_config.clone();

        // If connection settings changed and we're connected, disconnect and reconnect
        if (new_config.host != current_config.host
            || new_config.port != current_config.port
            || new_config.password != current_config.password)
            && self.connected.load(Ordering::SeqCst)
        {
            println!("OBS connection settings changed, reconnecting...");
            self.disconnect().await?;

            // Wait a bit before reconnecting
            sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;

            // Only reconnect if auto_connect is true
            if new_config.auto_connect {
                self.connect().await?;
            }
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
