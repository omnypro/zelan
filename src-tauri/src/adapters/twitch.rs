use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::adapters::{BaseAdapter, ServiceAdapter};
use crate::auth::token::{AuthEvent, AuthState, TokenData};
use crate::auth::AuthService;
use crate::events::EventBus;

// Constants for Twitch API
const TWITCH_API_BASE_URL: &str = "https://api.twitch.tv/helix";
const POLL_INTERVAL_SECS: u64 = 60; // 1 minute

/// Configuration for the Twitch adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchConfig {
    /// The Twitch channel to monitor
    #[serde(default)]
    pub channel_name: Option<String>,
    /// Whether to use EventSub for events
    #[serde(default = "default_use_eventsub")]
    pub use_eventsub: bool,
    /// Polling interval in seconds for regular API calls
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

fn default_use_eventsub() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    POLL_INTERVAL_SECS
}

impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            channel_name: None,
            use_eventsub: true,
            poll_interval_secs: POLL_INTERVAL_SECS,
        }
    }
}

/// Twitch API client
struct TwitchApiClient {
    client: Client,
    auth_service: Arc<AuthService>,
}

impl TwitchApiClient {
    /// Create a new Twitch API client
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            auth_service,
        }
    }
    
    /// Get a valid token for API requests
    async fn get_token(&self) -> Result<TokenData> {
        match self.auth_service.get_token("twitch").await? {
            Some(token) => {
                if token.is_valid() {
                    Ok(token)
                } else {
                    Err(anyhow!("Twitch token is expired"))
                }
            },
            None => Err(anyhow!("No Twitch token available")),
        }
    }
    
    /// Get user information for the authenticated user
    pub async fn get_user_info(&self) -> Result<serde_json::Value> {
        let token = self.get_token().await?;
        
        // https://dev.twitch.tv/docs/api/reference/#get-users
        let url = format!("{}/users", TWITCH_API_BASE_URL);
        
        let response = self.client
            .get(&url)
            .header("Client-ID", token.metadata.get("client_id").cloned().unwrap_or(json!("")))
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .context("Failed to send request to Twitch API")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Twitch API error: HTTP {}: {}", status, error_text));
        }
        
        let response_json = response.json::<serde_json::Value>().await
            .context("Failed to parse Twitch API response")?;
            
        Ok(response_json)
    }
    
    /// Get channel information
    pub async fn get_channel_info(&self, broadcaster_id: &str) -> Result<serde_json::Value> {
        let token = self.get_token().await?;
        
        // https://dev.twitch.tv/docs/api/reference/#get-channel-information
        let url = format!("{}/channels?broadcaster_id={}", TWITCH_API_BASE_URL, broadcaster_id);
        
        let response = self.client
            .get(&url)
            .header("Client-ID", token.metadata.get("client_id").cloned().unwrap_or(json!("")))
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .context("Failed to send request to Twitch API")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Twitch API error: HTTP {}: {}", status, error_text));
        }
        
        let response_json = response.json::<serde_json::Value>().await
            .context("Failed to parse Twitch API response")?;
            
        Ok(response_json)
    }
    
    /// Get stream information
    pub async fn get_stream_info(&self, user_id: &str) -> Result<serde_json::Value> {
        let token = self.get_token().await?;
        
        // https://dev.twitch.tv/docs/api/reference/#get-streams
        let url = format!("{}/streams?user_id={}", TWITCH_API_BASE_URL, user_id);
        
        let response = self.client
            .get(&url)
            .header("Client-ID", token.metadata.get("client_id").cloned().unwrap_or(json!("")))
            .header("Authorization", format!("Bearer {}", token.access_token))
            .send()
            .await
            .context("Failed to send request to Twitch API")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Twitch API error: HTTP {}: {}", status, error_text));
        }
        
        let response_json = response.json::<serde_json::Value>().await
            .context("Failed to parse Twitch API response")?;
            
        Ok(response_json)
    }
}

/// Twitch adapter implementation
pub struct TwitchAdapter {
    base: BaseAdapter,
    config: Arc<RwLock<TwitchConfig>>,
    twitch_api: Arc<TwitchApiClient>,
    auth_service: Arc<AuthService>,
    channel_id: Arc<RwLock<Option<String>>>,
    connected: Arc<RwLock<bool>>,
    polling_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    eventsub_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    auth_subscription: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    stream_status: Arc<RwLock<Option<serde_json::Value>>>,
    channel_info: Arc<RwLock<Option<serde_json::Value>>>,
}

impl TwitchAdapter {
    /// Create a new Twitch adapter
    pub fn new(event_bus: Arc<EventBus>, auth_service: Arc<AuthService>) -> Self {
        let twitch_api = Arc::new(TwitchApiClient::new(auth_service.clone()));
        
        Self {
            base: BaseAdapter::new("twitch", event_bus),
            config: Arc::new(RwLock::new(TwitchConfig::default())),
            twitch_api,
            auth_service,
            channel_id: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(false)),
            polling_task: Arc::new(RwLock::new(None)),
            eventsub_task: Arc::new(RwLock::new(None)),
            auth_subscription: Arc::new(RwLock::new(None)),
            stream_status: Arc::new(RwLock::new(None)),
            channel_info: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Subscribe to auth events
    async fn subscribe_to_auth_events(&self) {
        debug!("Subscribing to auth events");
        
        // Get a subscription
        let mut auth_sub = self.auth_service.subscribe();
        
        // Clean up any existing subscription task
        if let Some(handle) = self.auth_subscription.write().await.take() {
            handle.abort();
        }
        
        // Start listening for auth events
        let twitch_api = self.twitch_api.clone();
        let event_bus = self.base.event_bus.clone();
        let base_name = self.base.name().to_string();
        let connected = self.connected.clone();
        let handle = tokio::spawn(async move {
            while let Ok(event) = auth_sub.recv().await {
                match event {
                    AuthEvent::StateChanged { provider, state } if provider == "twitch" => {
                        match state {
                            AuthState::Authenticated { token } => {
                                // Publish auth status update event
                                let payload = json!({
                                    "status": "authenticated",
                                    "username": token.metadata.get("username").cloned().unwrap_or(json!("")),
                                    "user_id": token.user_id,
                                    "timestamp": Utc::now().to_rfc3339(),
                                });
                                
                                let _ = event_bus.publish_new(
                                    &base_name,
                                    "auth.status",
                                    payload,
                                ).await;
                                
                                // If already connected, refresh user info
                                if *connected.read().await {
                                    match twitch_api.get_user_info().await {
                                        Ok(user_info) => {
                                            let _ = event_bus.publish_new(
                                                &base_name,
                                                "user.updated",
                                                user_info,
                                            ).await;
                                        }
                                        Err(e) => {
                                            error!(error = %e, "Failed to refresh user info after auth update");
                                        }
                                    }
                                }
                            }
                            AuthState::Unauthenticated => {
                                // Publish auth status update event
                                let payload = json!({
                                    "status": "unauthenticated",
                                    "timestamp": Utc::now().to_rfc3339(),
                                });
                                
                                let _ = event_bus.publish_new(
                                    &base_name,
                                    "auth.status",
                                    payload,
                                ).await;
                                
                                // If connected, might need to disconnect
                                if *connected.read().await {
                                    warn!("Twitch token has been removed while adapter is connected");
                                    // Don't actually disconnect here - let the polling task fail naturally
                                }
                            }
                            AuthState::Failed { reason, .. } => {
                                // Publish auth status update event
                                let payload = json!({
                                    "status": "failed",
                                    "reason": reason,
                                    "timestamp": Utc::now().to_rfc3339(),
                                });
                                
                                let _ = event_bus.publish_new(
                                    &base_name,
                                    "auth.status",
                                    payload,
                                ).await;
                            }
                            _ => {} // Ignore other states
                        }
                    }
                    _ => {} // Ignore other events
                }
            }
        });
        
        // Store the subscription task
        *self.auth_subscription.write().await = Some(handle);
    }
    
    /// Start polling for Twitch data
    async fn start_polling(&self) -> Result<()> {
        debug!("Starting Twitch data polling");
        
        // Clean up any existing polling task
        if let Some(handle) = self.polling_task.write().await.take() {
            handle.abort();
        }
        
        // Get the poll interval from config
        let poll_interval_secs = self.config.read().await.poll_interval_secs;
        
        // First, get user information to determine channel ID
        let user_info = self.twitch_api.get_user_info().await?;
        
        // Extract user ID
        let user_id = user_info["data"][0]["id"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get user ID from Twitch API response"))?
            .to_string();
            
        // Store user ID
        *self.channel_id.write().await = Some(user_id.clone());
        
        // Publish initial user info event
        self.base.publish_event("user.info", user_info.clone()).await?;
        
        // Get initial state
        if let Ok(channel_info) = self.twitch_api.get_channel_info(&user_id).await {
            *self.channel_info.write().await = Some(channel_info.clone());
            self.base.publish_event("channel.info", channel_info).await?;
        }
        
        if let Ok(stream_info) = self.twitch_api.get_stream_info(&user_id).await {
            *self.stream_status.write().await = Some(stream_info.clone());
            self.base.publish_event("stream.info", stream_info).await?;
        }
        
        // Start polling tasks
        let twitch_api = self.twitch_api.clone();
        let event_bus = self.base.event_bus.clone();
        let base_name = self.base.name().to_string();
        let channel_id = user_id.clone();
        let stream_status = self.stream_status.clone();
        let channel_info = self.channel_info.clone();
        
        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(poll_interval_secs));
            
            loop {
                ticker.tick().await;
                
                // Poll for stream info
                match twitch_api.get_stream_info(&channel_id).await {
                    Ok(stream_info) => {
                        // Check if stream status has changed
                        let stream_changed = {
                            let old_status = stream_status.read().await;
                            if let Some(old) = &*old_status {
                                // Check if stream is online now vs before
                                let was_online = !old["data"].as_array().unwrap_or(&vec![]).is_empty();
                                let is_online = !stream_info["data"].as_array().unwrap_or(&vec![]).is_empty();
                                
                                // If online status changed, that's a major change
                                if was_online != is_online {
                                    true
                                } else if is_online {
                                    // Both online, check for title/category changes
                                    let old_stream = &old["data"][0];
                                    let new_stream = &stream_info["data"][0];
                                    
                                    let old_title = old_stream["title"].as_str().unwrap_or("");
                                    let new_title = new_stream["title"].as_str().unwrap_or("");
                                    
                                    let old_game = old_stream["game_name"].as_str().unwrap_or("");
                                    let new_game = new_stream["game_name"].as_str().unwrap_or("");
                                    
                                    old_title != new_title || old_game != new_game
                                } else {
                                    // Both offline, no change
                                    false
                                }
                            } else {
                                // No previous status, consider it changed
                                true
                            }
                        };
                        
                        // If changed, update and emit event
                        if stream_changed {
                            *stream_status.write().await = Some(stream_info.clone());
                            
                            // Determine if stream went online or offline
                            let is_online = !stream_info["data"].as_array().unwrap_or(&vec![]).is_empty();
                            
                            if is_online {
                                // Stream is online
                                let stream_data = &stream_info["data"][0];
                                
                                // Extract relevant fields
                                let stream_payload = json!({
                                    "status": "online",
                                    "title": stream_data["title"],
                                    "game_name": stream_data["game_name"],
                                    "viewer_count": stream_data["viewer_count"],
                                    "started_at": stream_data["started_at"],
                                    "thumbnail_url": stream_data["thumbnail_url"],
                                    "tags": stream_data["tags"],
                                    "timestamp": Utc::now().to_rfc3339(),
                                });
                                
                                // Publish stream.online event
                                let _ = event_bus.publish_new(
                                    &base_name,
                                    "stream.online",
                                    stream_payload,
                                ).await;
                            } else {
                                // Stream is offline
                                let stream_payload = json!({
                                    "status": "offline",
                                    "timestamp": Utc::now().to_rfc3339(),
                                });
                                
                                // Publish stream.offline event
                                let _ = event_bus.publish_new(
                                    &base_name,
                                    "stream.offline",
                                    stream_payload,
                                ).await;
                            }
                            
                            // Always publish the general stream.info event
                            let _ = event_bus.publish_new(
                                &base_name,
                                "stream.info",
                                stream_info,
                            ).await;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to poll for stream info");
                        
                        // Publish error event
                        let error_payload = json!({
                            "error": e.to_string(),
                            "component": "polling",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        
                        let _ = event_bus.publish_new(
                            &base_name,
                            "error",
                            error_payload,
                        ).await;
                    }
                }
                
                // Poll for channel info
                match twitch_api.get_channel_info(&channel_id).await {
                    Ok(channel_info_new) => {
                        // Check if channel info has changed
                        let channel_changed = {
                            let old_info = channel_info.read().await;
                            if let Some(old) = &*old_info {
                                let old_channel = &old["data"][0];
                                let new_channel = &channel_info_new["data"][0];
                                
                                let old_title = old_channel["title"].as_str().unwrap_or("");
                                let new_title = new_channel["title"].as_str().unwrap_or("");
                                
                                let old_game = old_channel["game_name"].as_str().unwrap_or("");
                                let new_game = new_channel["game_name"].as_str().unwrap_or("");
                                
                                old_title != new_title || old_game != new_game
                            } else {
                                // No previous info, consider it changed
                                true
                            }
                        };
                        
                        // If changed, update and emit event
                        if channel_changed {
                            *channel_info.write().await = Some(channel_info_new.clone());
                            
                            // Extract channel data
                            let channel_data = &channel_info_new["data"][0];
                            
                            // Create payload with relevant fields
                            let channel_payload = json!({
                                "title": channel_data["title"],
                                "game_name": channel_data["game_name"],
                                "game_id": channel_data["game_id"],
                                "broadcaster_language": channel_data["broadcaster_language"],
                                "tags": channel_data["tags"],
                                "timestamp": Utc::now().to_rfc3339(),
                            });
                            
                            // Publish channel.updated event
                            let _ = event_bus.publish_new(
                                &base_name,
                                "channel.updated",
                                channel_payload,
                            ).await;
                            
                            // Also publish the general channel.info event
                            let _ = event_bus.publish_new(
                                &base_name,
                                "channel.info",
                                channel_info_new,
                            ).await;
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to poll for channel info");
                        
                        // Publish error event
                        let error_payload = json!({
                            "error": e.to_string(),
                            "component": "polling",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        
                        let _ = event_bus.publish_new(
                            &base_name,
                            "error",
                            error_payload,
                        ).await;
                    }
                }
            }
        });
        
        // Store the polling task
        *self.polling_task.write().await = Some(handle);
        
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TwitchAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    async fn connect(&self) -> Result<()> {
        info!("Connecting Twitch adapter");
        
        // Check if already connected
        if *self.connected.read().await {
            return Ok(());
        }
        
        // Update status to connecting
        self.base.set_status(crate::adapters::AdapterStatus::Connecting).await;
        
        // Check for authentication
        let auth_state = self.auth_service.get_auth_state("twitch").await
            .map_err(|_| anyhow!("Failed to get Twitch auth state"))?;
            
        match auth_state {
            AuthState::Authenticated { .. } => {
                // Start subscription to auth events
                self.subscribe_to_auth_events().await;
                
                // Start polling for data
                self.start_polling().await?;
                
                // Update status and connected flag
                self.base.set_status(crate::adapters::AdapterStatus::Connected).await;
                *self.connected.write().await = true;
                
                Ok(())
            }
            _ => {
                self.base.set_status(crate::adapters::AdapterStatus::Error).await;
                Err(anyhow!("Twitch authentication required before connecting"))
            }
        }
    }
    
    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting Twitch adapter");
        
        // Check if already disconnected
        if !*self.connected.read().await {
            return Ok(());
        }
        
        // Stop polling task
        if let Some(handle) = self.polling_task.write().await.take() {
            handle.abort();
        }
        
        // Stop EventSub task
        if let Some(handle) = self.eventsub_task.write().await.take() {
            handle.abort();
        }
        
        // Stop auth subscription
        if let Some(handle) = self.auth_subscription.write().await.take() {
            handle.abort();
        }
        
        // Update status and connected flag
        self.base.set_status(crate::adapters::AdapterStatus::Disconnected).await;
        *self.connected.write().await = false;
        
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        // We need to block here to get the value from the RwLock
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.connected.read().await
            })
        })
    }
    
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        debug!(config = ?config, "Configuring Twitch adapter");
        
        // Convert to our config type
        let new_config = serde_json::from_value::<TwitchConfig>(config)?;
        
        // Store the config
        *self.config.write().await = new_config;
        
        Ok(())
    }
    
    async fn get_metrics(&self) -> Result<serde_json::Value> {
        // Get metrics about the adapter
        let metrics = json!({
            "connected": *self.connected.read().await,
            "channel_id": self.channel_id.read().await.clone(),
            "polling_active": self.polling_task.read().await.is_some(),
            "eventsub_active": self.eventsub_task.read().await.is_some(),
            "auth_subscription_active": self.auth_subscription.read().await.is_some(),
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        Ok(metrics)
    }
}