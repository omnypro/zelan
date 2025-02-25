use crate::{
    adapters::base::{AdapterConfig, BaseAdapter},
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use twitch_oauth2::{Scope, UserToken};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};

use super::twitch_auth::{AuthEvent, TwitchAuthManager};
use super::twitch_api::TwitchApiClient;

/// Environment variable name for Twitch Client ID
const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

/// Get the Twitch Client ID from environment
fn get_client_id() -> Result<String> {
    match env::var(TWITCH_CLIENT_ID_ENV) {
        Ok(client_id) if !client_id.is_empty() => Ok(client_id),
        Ok(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is empty")),
        Err(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is not set")),
    }
}

/// Default poll interval in milliseconds
const DEFAULT_POLL_INTERVAL_MS: u64 = 30000; // 30 seconds
/// Default device code polling interval in milliseconds
const DEVICE_CODE_POLLING_MS: u64 = 5000; // 5 seconds

/// Configuration for the Twitch adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchConfig {
    /// Twitch Channel ID to monitor
    pub channel_id: Option<String>,
    /// Twitch Channel Login (username) to monitor
    pub channel_login: Option<String>,
    /// Access token (if already authenticated)
    pub access_token: Option<String>,
    /// Refresh token (if already authenticated)
    pub refresh_token: Option<String>,
    /// Event poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Whether to monitor channel info
    pub monitor_channel_info: bool,
    /// Whether to monitor stream status
    pub monitor_stream_status: bool,
}

impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            channel_id: None,
            channel_login: None,
            access_token: None,
            refresh_token: None,
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            monitor_channel_info: true,
            monitor_stream_status: true,
        }
    }
}

impl AdapterConfig for TwitchConfig {
    fn to_json(&self) -> Value {
        json!({
            "channel_id": self.channel_id,
            "channel_login": self.channel_login,
            "access_token": self.access_token,
            "refresh_token": self.refresh_token,
            "poll_interval_ms": self.poll_interval_ms,
            "monitor_channel_info": self.monitor_channel_info,
            "monitor_stream_status": self.monitor_stream_status,
        })
    }
    
    fn from_json(json: &Value) -> Result<Self> {
        let mut config = TwitchConfig::default();
        
        // Extract channel ID
        if let Some(channel_id) = json.get("channel_id").and_then(|v| v.as_str()) {
            config.channel_id = Some(channel_id.to_string());
        }
        
        // Extract channel login
        if let Some(channel_login) = json.get("channel_login").and_then(|v| v.as_str()) {
            config.channel_login = Some(channel_login.to_string());
        }
        
        // Extract access token
        if let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) {
            config.access_token = Some(access_token.to_string());
        }
        
        // Extract refresh token
        if let Some(refresh_token) = json.get("refresh_token").and_then(|v| v.as_str()) {
            config.refresh_token = Some(refresh_token.to_string());
        }
        
        // Extract poll interval
        if let Some(poll_interval) = json.get("poll_interval_ms").and_then(|v| v.as_u64()) {
            // Ensure interval is reasonable (minimum 5000ms, maximum 300000ms)
            config.poll_interval_ms = poll_interval.clamp(5000, 300000);
        }
        
        // Extract monitor flags
        if let Some(monitor_channel) = json.get("monitor_channel_info").and_then(|v| v.as_bool()) {
            config.monitor_channel_info = monitor_channel;
        }
        
        if let Some(monitor_stream) = json.get("monitor_stream_status").and_then(|v| v.as_bool()) {
            config.monitor_stream_status = monitor_stream;
        }
        
        Ok(config)
    }
    
    fn adapter_type() -> &'static str {
        "twitch"
    }
    
    fn validate(&self) -> Result<()> {
        // Need either channel ID or channel login
        if self.channel_id.is_none() && self.channel_login.is_none() {
            return Err(anyhow!("Either Channel ID or Channel Login must be provided"));
        }
        
        // Ensure poll interval is reasonable
        if self.poll_interval_ms < 5000 || self.poll_interval_ms > 300000 {
            return Err(anyhow!("Poll interval must be between 5000ms and 300000ms"));
        }
        
        Ok(())
    }
}

/// Internal state for tracking previous API responses
#[derive(Debug, Clone)]
struct TwitchState {
    /// Last fetched channel information
    last_channel_info: Option<ChannelInformation>,
    /// Last fetched stream information
    last_stream_info: Option<Stream>,
    /// Whether stream was live in last poll
    was_live: bool,
}

impl Default for TwitchState {
    fn default() -> Self {
        Self {
            last_channel_info: None,
            last_stream_info: None,
            was_live: false,
        }
    }
}

/// Twitch adapter for connecting to Twitch's API
pub struct TwitchAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// Configuration specific to the TwitchAdapter
    config: Arc<RwLock<TwitchConfig>>,
    /// Authentication manager
    auth_manager: Arc<RwLock<TwitchAuthManager>>,
    /// API client
    api_client: TwitchApiClient,
    /// State tracking for events
    state: Arc<RwLock<TwitchState>>,
}

impl TwitchAdapter {
    /// Create a new Twitch adapter
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new Twitch adapter");
        
        // Get the client ID from environment
        let client_id = match get_client_id() {
            Ok(id) => {
                info!("Using Twitch Client ID from environment variable");
                id
            },
            Err(e) => {
                error!("Failed to get Twitch Client ID from environment: {}", e);
                String::new()
            }
        };
        
        Self {
            base: BaseAdapter::new("twitch", event_bus.clone()),
            config: Arc::new(RwLock::new(TwitchConfig::default())),
            auth_manager: Arc::new(RwLock::new(TwitchAuthManager::new(
                client_id,
            ))),
            api_client: TwitchApiClient::new(),
            state: Arc::new(RwLock::new(TwitchState::default())),
        }
    }
    
    /// Create a new Twitch adapter with custom config
    #[instrument(skip(event_bus), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: TwitchConfig) -> Self {
        info!(
            channel_id = ?config.channel_id,
            channel_login = ?config.channel_login,
            "Creating Twitch adapter with custom config"
        );
        
        // Get the client ID from environment
        let client_id = match get_client_id() {
            Ok(id) => {
                info!("Using Twitch Client ID from environment variable");
                id
            },
            Err(e) => {
                error!("Failed to get Twitch Client ID from environment: {}", e);
                String::new()
            }
        };
        
        // Create the adapter with the base configuration
        let adapter = Self {
            base: BaseAdapter::new("twitch", event_bus.clone()),
            config: Arc::new(RwLock::new(config.clone())),
            auth_manager: Arc::new(RwLock::new(TwitchAuthManager::new(
                client_id
            ))),
            api_client: TwitchApiClient::new(),
            state: Arc::new(RwLock::new(TwitchState::default())),
        };
        
        // Set up auth callback
        let event_bus_clone = event_bus.clone();
        {
            // Create a scope to ensure the lock is released after setting the callback
            let mut auth_manager = adapter.auth_manager.blocking_write();
            auth_manager.set_auth_callback(move |event| -> Result<()> {
            let event_type = match &event {
                AuthEvent::DeviceCodeReceived { .. } => "device_code",
                AuthEvent::AuthenticationSuccess => "success",
                AuthEvent::AuthenticationFailed { .. } => "failed",
                AuthEvent::TokenRefreshed => "token_refreshed",
                AuthEvent::TokenExpired { .. } => "token_expired",
            };
            
            let payload = match event {
                AuthEvent::DeviceCodeReceived { 
                    verification_uri, 
                    user_code, 
                    expires_in 
                } => {
                    json!({
                        "event": "device_code_received",
                        "verification_uri": verification_uri,
                        "user_code": user_code,
                        "expires_in": expires_in,
                        "message": "Please visit the verification URL and enter the code to authenticate",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })
                },
                AuthEvent::AuthenticationSuccess => {
                    json!({
                        "event": "authentication_success",
                        "message": "Successfully authenticated with Twitch",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })
                },
                AuthEvent::AuthenticationFailed { error } => {
                    json!({
                        "event": "authentication_failed",
                        "error": error,
                        "message": "Failed to authenticate with Twitch",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })
                },
                AuthEvent::TokenRefreshed => {
                    json!({
                        "event": "token_refreshed",
                        "message": "Successfully refreshed Twitch token",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })
                },
                AuthEvent::TokenExpired { error } => {
                    json!({
                        "event": "token_expired",
                        "message": "Token refresh failed, need to re-authenticate",
                        "error": error,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })
                },
            };
            
            // Since we can't use await in this callback, we'll spawn a task to publish
            let event_type_str = format!("auth.{}", event_type);
            let event = StreamEvent::new(
                "twitch", 
                &event_type_str,
                payload
            );
            
            // Spawn a task to publish the event
            let event_bus = event_bus_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = event_bus.publish(event).await {
                    error!("Failed to publish event: {}", e);
                }
            });
            
            Ok(())
        });
        } // End of auth_manager scope - lock is released here
        
        // Restore from saved tokens if available
        if let (Some(access_token), refresh_token) = (
            &config.access_token, 
            &config.refresh_token
        ) {
            info!("Attempting to restore authentication from saved tokens");
            
            // Try to restore in a non-blocking way (spawn a task)
            let adapter_clone = adapter.clone();
            let access_token_clone = access_token.clone();
            let refresh_token_clone = refresh_token.clone();
            tokio::spawn(async move {
                let result = adapter_clone.auth_manager.write().await.restore_from_saved_tokens(
                    access_token_clone,
                    refresh_token_clone,
                ).await;
                
                match result {
                    Ok(_) => {
                        info!("Successfully restored and validated authentication tokens");
                    },
                    Err(e) => {
                        // Token validation failed, we need to notify the user
                        warn!("Failed to restore from saved tokens (invalid or expired): {}", e);
                        
                        // Clear tokens from config since they're invalid
                        let mut config = adapter_clone.config.write().await;
                        config.access_token = None;
                        config.refresh_token = None;
                        
                        // Publish an event so the UI can notify the user
                        let payload = json!({
                            "event": "token_invalid",
                            "message": "Saved authentication tokens are invalid or expired. Please re-authenticate.",
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        
                        // Publish via base adapter
                        if let Err(event_err) = adapter_clone.base.publish_event("auth.token_invalid", payload).await {
                            error!("Failed to publish token invalid event: {}", event_err);
                        }
                    }
                }
            });
        }
        
        adapter
    }
    
    /// Convert a JSON config to a TwitchConfig
    #[instrument(skip(config_json), level = "debug")]
    pub fn config_from_json(config_json: &Value) -> Result<TwitchConfig> {
        TwitchConfig::from_json(config_json)
    }
    
    /// Start device auth flow and wait for completion
    async fn authenticate(&self) -> Result<()> {
        let auth_manager = self.auth_manager.read().await;
        
        // Start the device code flow with empty vector
        // The auth manager handles the scope selection internally
        let device_code = auth_manager.start_device_auth(vec![]).await?;
        
        // Release the auth_manager lock before spawning the polling task
        drop(auth_manager);
        
        // Create clones for use in the task
        let auth_manager_clone = Arc::clone(&self.auth_manager);
        let config_clone = Arc::clone(&self.config);
        
        // Start the polling in a separate task
        tokio::spawn(async move {
            loop {
                // Poll for completion
                match auth_manager_clone.read().await.poll_device_auth().await {
                    Ok(token) => {
                        info!("Device auth completed successfully");
                        
                        // Update config with the tokens
                        if let Some((access_token, refresh_token)) = 
                            auth_manager_clone.read().await.get_token_for_storage().await {
                            
                            let mut config = config_clone.write().await;
                            config.access_token = Some(access_token);
                            config.refresh_token = refresh_token;
                        }
                        
                        break;
                    },
                    Err(e) => {
                        if e.to_string().contains("authorization_pending") {
                            // Still waiting, continue polling
                            tokio::time::sleep(Duration::from_millis(DEVICE_CODE_POLLING_MS)).await;
                            continue;
                        } else {
                            // Real error
                            error!("Device auth polling failed: {}", e);
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Poll for Twitch channel and stream updates
    #[instrument(skip(self, shutdown_rx), level = "debug")]
    async fn poll_twitch_updates(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        info!("Starting Twitch update polling");
        
        // Loop until shutdown or disconnected
        loop {
            // Check for shutdown signal with a short timeout
            let maybe_shutdown = tokio::time::timeout(
                Duration::from_millis(10),
                shutdown_rx.recv()
            ).await;
            
            if maybe_shutdown.is_ok() {
                info!("Received shutdown signal for Twitch polling");
                break;
            }
            
            // Check if we're still connected
            if !self.base.is_connected() {
                info!("Twitch adapter no longer connected, stopping polling");
                break;
            }
            
            // Check authentication
            let auth_manager = self.auth_manager.read().await;
            if !auth_manager.is_authenticated().await {
                // Need to authenticate
                drop(auth_manager); // Release the lock
                
                if let Err(e) = self.authenticate().await {
                    error!("Failed to start authentication: {}", e);
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
                
                // Short wait before continuing
                tokio::time::sleep(Duration::from_millis(1000)).await;
                continue;
            }
            
            // Get the token
            let token = match auth_manager.get_token().await {
                Some(token) => token,
                None => {
                    // This shouldn't happen as we just checked is_authenticated
                    error!("No token available despite being authenticated");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            
            // Check if token needs refresh
            if let Err(e) = auth_manager.refresh_token_if_needed().await {
                // If error contains 'Not authenticated', it means the token was likely expired
                // and has already been reset in the auth manager
                if e.to_string().contains("Not authenticated") {
                    info!("Not authenticated or token expired, will restart authentication process");
                } else {
                    error!("Failed to refresh token: {}", e);
                }
                
                // Continue to next iteration to authenticate again
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            
            // Drop the auth manager lock
            drop(auth_manager);
            
            // Get config
            let config = self.config.read().await.clone();
            
            // Fetch channel info if enabled
            if config.monitor_channel_info {
                if let Some(channel_id) = &config.channel_id {
                    match self.api_client.fetch_channel_info(&token, channel_id).await {
                        Ok(Some(channel_info)) => {
                            // Check if channel info has changed
                            let mut state = self.state.write().await;
                            if state.last_channel_info.as_ref() != Some(&channel_info) {
                                // Channel info changed, publish event
                                debug!("Channel info changed, publishing event");
                                
                                // Store new info
                                state.last_channel_info = Some(channel_info.clone());
                                
                                // Convert to json value for event
                                let channel_json = serde_json::to_value(&channel_info)?;
                                
                                // Publish event
                                let payload = json!({
                                    "channel": channel_json,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                });
                                
                                self.base.publish_event("channel.updated", payload).await?;
                            }
                        },
                        Ok(None) => {
                            debug!("No channel information found");
                        },
                        Err(e) => {
                            error!(error = %e, "Failed to fetch channel info");
                        }
                    }
                } else if let Some(login) = &config.channel_login {
                    // TODO: Implement lookup by login
                    warn!("Lookup by channel login not implemented yet");
                }
            }
            
            // Fetch stream info if enabled
            if config.monitor_stream_status {
                match self.api_client.fetch_stream_info(
                    &token,
                    config.channel_id.as_deref(),
                    config.channel_login.as_deref(),
                ).await {
                    Ok(Some(stream_info)) => {
                        // Stream is live
                        let mut state = self.state.write().await;
                        
                        // Check if we need to publish a stream.online event
                        if !state.was_live {
                            // Stream just went online
                            info!("Stream went online, publishing event");
                            
                            // Convert to json value for event
                            let stream_json = serde_json::to_value(&stream_info)?;
                            
                            // Publish stream online event
                            let payload = json!({
                                "stream": stream_json,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            self.base.publish_event("stream.online", payload).await?;
                            state.was_live = true;
                        }
                        
                        // Check if stream info has changed
                        if state.last_stream_info.as_ref() != Some(&stream_info) {
                            // Stream info changed, publish event
                            debug!("Stream info changed, publishing event");
                            
                            // Store new info
                            state.last_stream_info = Some(stream_info.clone());
                            
                            // Convert to json value for event
                            let stream_json = serde_json::to_value(&stream_info)?;
                            
                            // Publish event
                            let payload = json!({
                                "stream": stream_json,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            self.base.publish_event("stream.updated", payload).await?;
                        }
                    },
                    Ok(None) => {
                        // Stream is offline
                        let mut state = self.state.write().await;
                        
                        // Check if we need to publish a stream.offline event
                        if state.was_live {
                            // Stream just went offline
                            info!("Stream went offline, publishing event");
                            
                            // Publish stream offline event
                            let payload = json!({
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            self.base.publish_event("stream.offline", payload).await?;
                            state.was_live = false;
                            state.last_stream_info = None;
                        }
                    },
                    Err(e) => {
                        error!(error = %e, "Failed to fetch stream info");
                    }
                }
            }
            
            // Wait for next poll interval
            tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)).await;
        }
        
        info!("Twitch update polling stopped");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TwitchAdapter {
    #[instrument(skip(self), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Only connect if not already connected
        if self.base.is_connected() {
            info!("Twitch adapter is already connected");
            return Ok(());
        }
        
        info!("Connecting Twitch adapter");
        
        // Create the shutdown channel
        let (_, shutdown_rx) = self.base.create_shutdown_channel().await;
        
        // Set connected state
        self.base.set_connected(true);
        
        // Start polling in a background task
        let self_clone = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.poll_twitch_updates(shutdown_rx).await {
                error!(error = %e, "Error in Twitch update polling");
            }
        });
        
        // Store the event handler task
        self.base.set_event_handler(handle).await;
        
        info!("Twitch adapter connected and polling for updates");
        Ok(())
    }
    
    #[instrument(skip(self), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Only disconnect if connected
        if !self.base.is_connected() {
            debug!("Twitch adapter is already disconnected");
            return Ok(());
        }
        
        info!("Disconnecting Twitch adapter");
        
        // Set disconnected state to stop event generation
        self.base.set_connected(false);
        
        // Stop the event handler
        self.base.stop_event_handler().await?;
        
        info!("Twitch adapter disconnected");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }
    
    fn get_name(&self) -> &str {
        self.base.name()
    }
    
    #[instrument(skip(self, config), level = "debug")]
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        info!("Configuring Twitch adapter");
        
        // Parse the config
        let new_config = TwitchConfig::from_json(&config)?;
        
        // Validate the new configuration
        new_config.validate()?;
        
        // Check if authentication info changed
        let current_config = self.config.read().await.clone();
        let auth_changed = current_config.access_token != new_config.access_token
            || current_config.refresh_token != new_config.refresh_token;
        
        // Check if channel changed
        let channel_changed = current_config.channel_id != new_config.channel_id
            || current_config.channel_login != new_config.channel_login;
        
        // Update our configuration
        let mut current_config = self.config.write().await;
        *current_config = new_config.clone();
        
        // If auth changed, update auth manager
        if auth_changed {
            info!("Authentication info changed, updating auth manager");
            
            // Get the client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => {
                    info!("Using Twitch Client ID from environment variable");
                    id
                },
                Err(e) => {
                    error!("Failed to get Twitch Client ID from environment: {}", e);
                    String::new()
                }
            };
            
            // Update auth manager
            let mut auth_manager = self.auth_manager.write().await;
            *auth_manager = TwitchAuthManager::new(client_id);
            
            // If we have access token, restore it
            if let Some(access_token) = &new_config.access_token {
                info!("Attempting to restore authentication from saved tokens");
                
                let result = auth_manager.restore_from_saved_tokens(
                    access_token.clone(),
                    new_config.refresh_token.clone(),
                ).await;
                
                match result {
                    Ok(_) => {
                        info!("Successfully restored and validated authentication tokens");
                    },
                    Err(e) => {
                        // Token validation failed, we need to notify the user
                        warn!("Failed to restore from saved tokens (invalid or expired): {}", e);
                        
                        // Clear tokens from config since they're invalid
                        let mut config = self.config.write().await;
                        config.access_token = None;
                        config.refresh_token = None;
                        
                        // Publish an event so the UI can notify the user
                        let payload = json!({
                            "event": "token_invalid",
                            "message": "Saved authentication tokens are invalid or expired. Please re-authenticate.",
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        
                        if let Err(event_err) = self.base.publish_event("auth.token_invalid", payload).await {
                            error!("Failed to publish token invalid event: {}", event_err);
                        }
                    }
                }
            }
        }
        
        // If channel changed, reset state
        if channel_changed {
            info!("Channel changed, resetting state");
            *self.state.write().await = TwitchState::default();
        }
        
        info!(
            channel_id = ?new_config.channel_id,
            channel_login = ?new_config.channel_login,
            poll_interval_ms = new_config.poll_interval_ms,
            "Twitch adapter configured"
        );
        
        Ok(())
    }
}

impl Clone for TwitchAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with the same event bus
        let event_bus = self.base.event_bus();
        
        // Since we're using Arc<RwLock<>>, we can just clone those directly
        Self {
            base: self.base.clone(),
            config: Arc::clone(&self.config),
            auth_manager: Arc::clone(&self.auth_manager),
            api_client: self.api_client.clone(),
            state: Arc::clone(&self.state),
        }
    }
}
