use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};
use twitch_api::{helix, TwitchClient};
use twitch_oauth2::{AppAccessToken, ClientId, ClientSecret};

use crate::adapters::ServiceAdapter;
use crate::core::{EventBus, StreamEvent};
use crate::error::Error;

/// Configuration for the Twitch adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchConfig {
    /// The Twitch channel to monitor
    pub channel_name: Option<String>,
    /// Polling interval for checking stream status (in seconds)
    pub polling_interval: u64,
}

impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            channel_name: None,
            polling_interval: 60, // Check every 60 seconds by default
        }
    }
}

/// Adapter for connecting to Twitch APIs
#[derive(Debug)]
pub struct TwitchAdapter {
    /// Name of the adapter
    name: String,
    /// Connected status
    connected: Arc<RwLock<bool>>,
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Configuration
    config: Arc<RwLock<TwitchConfig>>,
    /// Twitch client
    client: Arc<TwitchClient<reqwest::Client>>,
    /// App access token for Twitch API
    token: Arc<RwLock<Option<AppAccessToken>>>,
    /// Cached user ID for the configured channel
    user_id: Arc<RwLock<Option<String>>>,
    /// Task handle for the background worker
    worker_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Shutdown sender for stopping the background worker
    shutdown_sender: Arc<RwLock<Option<tokio::sync::mpsc::Sender<()>>>>,
}

impl TwitchAdapter {
    /// Create a new Twitch adapter
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new Twitch adapter");
        let http_client = Client::new();
        let client = TwitchClient::new(http_client);
        
        Self {
            name: "twitch".to_string(),
            connected: Arc::new(RwLock::new(false)),
            event_bus,
            config: Arc::new(RwLock::new(TwitchConfig::default())),
            client: Arc::new(client),
            token: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            worker_handle: Arc::new(RwLock::new(None)),
            shutdown_sender: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Create a new Twitch adapter with custom configuration
    #[instrument(skip(event_bus, config), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: TwitchConfig) -> Self {
        info!("Creating Twitch adapter with custom config");
        let http_client = Client::new();
        let client = TwitchClient::new(http_client);
        
        Self {
            name: "twitch".to_string(),
            connected: Arc::new(RwLock::new(false)),
            event_bus,
            config: Arc::new(RwLock::new(config)),
            client: Arc::new(client),
            token: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            worker_handle: Arc::new(RwLock::new(None)),
            shutdown_sender: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Get an app access token for Twitch API access
    #[instrument(skip(self), level = "debug")]
    async fn get_app_token(&self) -> Result<AppAccessToken> {
        // Check if we already have a valid token
        if let Some(token) = &*self.token.read().await {
            // If token is not close to expiration, return it
            if token.expires_in() > Duration::from_secs(300) {
                debug!("Using existing app token");
                return Ok(token.clone());
            }
        }
        
        // Load client ID and secret from environment variables
        let client_id = std::env::var("TWITCH_CLIENT_ID")
            .map_err(|_| Error::configuration("TWITCH_CLIENT_ID environment variable not set".to_string()))?;
        
        let client_secret = std::env::var("TWITCH_CLIENT_SECRET")
            .map_err(|_| Error::configuration("TWITCH_CLIENT_SECRET environment variable not set".to_string()))?;
        
        // Create ClientId and ClientSecret
        let client_id = ClientId::new(client_id);
        let client_secret = ClientSecret::new(client_secret);
        
        // Get a new token
        info!("Getting new Twitch app access token");
        let app_token = AppAccessToken::get_app_access_token(
            &self.client.inner_client().inner().clone(),
            client_id,
            client_secret,
            vec![], // No scopes needed for app token
        )
        .await
        .map_err(|e| {
            error!("Failed to get Twitch app access token: {}", e);
            Error::external(format!("Failed to get Twitch app access token: {}", e))
        })?;
        
        // Store the token
        *self.token.write().await = Some(app_token.clone());
        
        info!("Successfully obtained new Twitch app access token");
        Ok(app_token)
    }
    
    /// Get user ID from username
    #[instrument(skip(self, token), level = "debug")]
    async fn get_user_id(&self, token: &AppAccessToken, username: &str) -> Result<String> {
        // Check cache first
        if let Some(user_id) = &*self.user_id.read().await {
            return Ok(user_id.clone());
        }
        
        info!("Looking up Twitch user ID for {}", username);
        let response = self.client.helix
            .get_users(
                Some(&token.bearer_token()),
                &[helix::users::UserNameOrId::from(username)],
                None,
                None,
            )
            .await
            .map_err(|e| {
                error!("Failed to get user ID for {}: {}", username, e);
                Error::external(format!("Failed to get Twitch user ID: {}", e))
            })?;
        
        // Get the first user
        let user = response.data.first().ok_or_else(|| {
            error!("No Twitch user found with name {}", username);
            Error::external(format!("No Twitch user found with name {}", username))
        })?;
        
        // Store user ID in cache
        let user_id = user.id.clone();
        *self.user_id.write().await = Some(user_id.clone());
        
        info!("Found Twitch user ID {} for {}", user_id, username);
        Ok(user_id)
    }
    
    /// Check if a stream is live
    #[instrument(skip(self, token), level = "debug")]
    async fn check_stream_status(&self, token: &AppAccessToken, user_id: &str) -> Result<Option<Value>> {
        info!("Checking stream status for user ID {}", user_id);
        let response = self.client.helix
            .get_streams(
                Some(&token.bearer_token()),
                Some(&[&user_id]), // User IDs to query
                None,              // No user logins
                None,              // No game IDs
                None,              // No first parameter
                None,              // No type
                None,              // No language
            )
            .await
            .map_err(|e| {
                error!("Failed to check stream status: {}", e);
                Error::external(format!("Failed to check Twitch stream status: {}", e))
            })?;
        
        // If no streams are returned, the channel is offline
        if response.data.is_empty() {
            debug!("Stream is offline for user ID {}", user_id);
            return Ok(None);
        }
        
        // Stream is online, return details as JSON
        let stream = response.data.first().unwrap();
        let stream_data = json!({
            "id": stream.id,
            "user_id": stream.user_id,
            "user_name": stream.user_name,
            "game_id": stream.game_id,
            "game_name": stream.game_name,
            "type": stream.type_field,
            "title": stream.title,
            "viewer_count": stream.viewer_count,
            "started_at": stream.started_at,
            "language": stream.language,
            "thumbnail_url": stream.thumbnail_url,
            "tag_ids": stream.tag_ids,
            "is_mature": stream.is_mature,
        });
        
        debug!("Stream is online for user ID {}", user_id);
        Ok(Some(stream_data))
    }
    
    /// Publish an event to the event bus
    async fn publish_event(&self, event_type: &str, payload: Value) -> Result<usize> {
        let event = StreamEvent::new("twitch", event_type, payload);
        self.event_bus.publish(event).await.map_err(|e| anyhow::anyhow!(e.to_string()))
    }
    
    /// Background worker task for polling Twitch API
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn worker_task(
        self: Arc<Self>,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> Result<()> {
        info!("Starting Twitch worker task");
        
        // Track previous stream status to detect changes
        let mut previous_status: Option<bool> = None;
        
        loop {
            // Check for shutdown signal
            let maybe_shutdown = tokio::time::timeout(
                tokio::time::Duration::from_millis(10),
                shutdown_rx.recv(),
            )
            .await;
            
            if maybe_shutdown.is_ok() {
                info!("Received shutdown signal for Twitch worker");
                break;
            }
            
            // Check if we're still connected
            if !*self.connected.read().await {
                info!("Twitch adapter no longer connected, stopping worker");
                break;
            }
            
            // Get current config
            let config = self.config.read().await.clone();
            
            // Skip this iteration if channel name is not set
            let Some(channel_name) = &config.channel_name else {
                warn!("Twitch channel name not configured, skipping status check");
                sleep(Duration::from_secs(config.polling_interval)).await;
                continue;
            };
            
            // Get app token
            match self.get_app_token().await {
                Ok(token) => {
                    // Get user ID
                    match self.get_user_id(&token, channel_name).await {
                        Ok(user_id) => {
                            // Check stream status
                            match self.check_stream_status(&token, &user_id).await {
                                Ok(stream_data) => {
                                    let is_live = stream_data.is_some();
                                    
                                    // If status changed, publish an event
                                    if previous_status != Some(is_live) {
                                        if is_live {
                                            // Stream went online
                                            info!("Stream went online for {}", channel_name);
                                            if let Some(data) = stream_data {
                                                if let Err(e) = self.publish_event("stream.online", data).await {
                                                    error!("Failed to publish stream.online event: {}", e);
                                                }
                                            }
                                        } else {
                                            // Stream went offline
                                            info!("Stream went offline for {}", channel_name);
                                            if let Err(e) = self.publish_event(
                                                "stream.offline",
                                                json!({
                                                    "user_id": user_id,
                                                    "user_name": channel_name,
                                                }),
                                            )
                                            .await
                                            {
                                                error!("Failed to publish stream.offline event: {}", e);
                                            }
                                        }
                                        
                                        // Update previous status
                                        previous_status = Some(is_live);
                                    }
                                }
                                Err(e) => {
                                    error!("Error checking stream status: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error getting user ID: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error getting app token: {}", e);
                }
            }
            
            // Wait for the polling interval
            sleep(Duration::from_secs(config.polling_interval)).await;
        }
        
        info!("Twitch worker task stopped");
        Ok(())
    }
    
    /// Convert a JSON config to TwitchConfig
    pub fn config_from_json(config_json: &Value) -> Result<TwitchConfig> {
        let mut config = TwitchConfig::default();
        
        // Extract channel name if provided
        if let Some(channel_name) = config_json.get("channel_name").and_then(|v| v.as_str()) {
            config.channel_name = Some(channel_name.to_string());
        }
        
        // Extract polling interval if provided
        if let Some(interval) = config_json.get("polling_interval").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 15s, maximum 300s)
            config.polling_interval = interval.clamp(15, 300);
        }
        
        Ok(config)
    }
}

#[async_trait]
impl ServiceAdapter for TwitchAdapter {
    #[instrument(skip(self), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Only connect if not already connected
        if *self.connected.read().await {
            info!("Twitch adapter is already connected");
            return Ok(());
        }
        
        info!("Connecting Twitch adapter");
        
        // Get current config and validate
        let config = self.config.read().await.clone();
        if config.channel_name.is_none() {
            return Err(anyhow::anyhow!("Twitch channel name not configured"));
        }
        
        // Test getting an app token to ensure we can connect
        let _token = self.get_app_token().await?;
        
        // Create the shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
        *self.shutdown_sender.write().await = Some(shutdown_tx);
        
        // Set connected state
        *self.connected.write().await = true;
        
        // Start the worker task
        let self_clone = Arc::new(self.clone());
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.worker_task(shutdown_rx).await {
                error!(error = %e, "Error in Twitch worker task");
            }
        });
        
        // Store the task handle
        *self.worker_handle.write().await = Some(handle);
        
        info!("Twitch adapter connected");
        Ok(())
    }
    
    #[instrument(skip(self), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if connected
        if !*self.connected.read().await {
            debug!("Twitch adapter is already disconnected");
            return Ok(());
        }
        
        info!("Disconnecting Twitch adapter");
        
        // Set disconnected state
        *self.connected.write().await = false;
        
        // Send shutdown signal
        if let Some(tx) = self.shutdown_sender.write().await.take() {
            let _ = tx.send(()).await;
            debug!("Sent shutdown signal to Twitch worker");
        }
        
        // Abort the task if it's still running
        if let Some(handle) = self.worker_handle.write().await.take() {
            handle.abort();
            debug!("Aborted Twitch worker task");
        }
        
        info!("Twitch adapter disconnected");
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
        info!("Configuring Twitch adapter");
        
        // Parse the config
        let new_config = Self::config_from_json(&config)?;
        
        // Update configuration
        *self.config.write().await = new_config.clone();
        
        info!(
            channel = ?new_config.channel_name,
            interval = new_config.polling_interval,
            "Twitch adapter configured"
        );
        
        Ok(())
    }
}

impl Clone for TwitchAdapter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            connected: self.connected.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
            client: self.client.clone(),
            token: self.token.clone(),
            user_id: self.user_id.clone(),
            worker_handle: self.worker_handle.clone(),
            shutdown_sender: self.shutdown_sender.clone(),
        }
    }
}