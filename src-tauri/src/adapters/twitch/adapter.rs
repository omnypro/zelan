use crate::{
    adapters::base::{BaseAdapter, ServiceHelperImpl},
    adapters::common::{AdapterError, TraceHelper},
    auth::token_manager::{TokenData, TokenManager},
    recovery::{AdapterRecovery, RecoveryManager},
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, env, sync::Arc};
use tauri::async_runtime::RwLock;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, trace, warn};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::TwitchToken;

use super::api::TwitchApiClient;
use super::auth::{AuthEvent, TwitchAuthManager};
use super::eventsub::EventSubClient;

/// Environment variable name for Twitch Client ID
const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

/// Get the Twitch Client ID from environment
fn get_client_id() -> Result<String, AdapterError> {
    match env::var(TWITCH_CLIENT_ID_ENV) {
        Ok(client_id) if !client_id.is_empty() => Ok(client_id),
        Ok(_) => Err(AdapterError::config(
            "TWITCH_CLIENT_ID environment variable is empty",
        )),
        Err(_) => Err(AdapterError::config(
            "TWITCH_CLIENT_ID environment variable is not set",
        )),
    }
}

/// Default poll interval in milliseconds
const DEFAULT_POLL_INTERVAL_MS: u64 = 30000; // 30 seconds
/// Default device code polling interval in milliseconds
const DEVICE_CODE_POLLING_MS: u64 = 5000; // 5 seconds
/// Device code polling maximum duration in seconds
const DEVICE_CODE_MAX_POLL_DURATION_SECS: u64 = 1800; // 30 minutes

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
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
}

/// Default poll interval
fn default_poll_interval() -> u64 {
    DEFAULT_POLL_INTERVAL_MS
}

/// Default implementation for TwitchConfig
impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            channel_id: None,
            channel_login: None,
            access_token: None,
            refresh_token: None,
            poll_interval_ms: default_poll_interval(),
        }
    }
}

/// Implementation of AdapterConfig for TwitchConfig
impl crate::adapters::base::AdapterConfig for TwitchConfig {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn from_json(json: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(json.clone())?)
    }

    fn adapter_type() -> &'static str {
        "twitch"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.channel_id.is_none() && self.channel_login.is_none() {
            anyhow::bail!("Either channel_id or channel_login must be provided");
        }
        Ok(())
    }
}

/// Helper to extract channel ID, login, or both
fn extract_channel_info(config: &TwitchConfig) -> (Option<String>, Option<String>) {
    let channel_id = config.channel_id.clone();
    let channel_login = config.channel_login.clone();

    (channel_id, channel_login)
}

/// The Twitch adapter for interacting with the Twitch API and EventSub
pub struct TwitchAdapter {
    /// Base adapter implementation - the foundation for all adapters
    base: BaseAdapter,
    /// Auth manager for handling Twitch authentication
    auth_manager: TwitchAuthManager,
    /// API client for Twitch API requests
    api_client: TwitchApiClient,
    /// EventSub client for real-time events
    eventsub_client: RwLock<Option<EventSubClient>>,
    /// Stream information cached from last poll
    stream_info: RwLock<Option<Stream>>,
    /// Channel information cached from last poll
    channel_info: RwLock<Option<ChannelInformation>>,
    /// Are we pending authentication completion?
    pending_auth: RwLock<bool>,
    /// Adapter config
    config: RwLock<TwitchConfig>,
    /// Stream ID of the last known stream
    last_stream_id: RwLock<Option<String>>,
    /// Helper for implementing ServiceAdapter
    service_helper: ServiceHelperImpl,
}

#[async_trait]
impl ServiceAdapter for TwitchAdapter {
    /// Get the adapter type
    fn adapter_type(&self) -> &'static str {
        "twitch"
    }

    /// Connect to the service
    async fn connect(&self) -> Result<(), AdapterError> {
        trace!("Twitch adapter connect() called");

        // Get service helper features
        let features = self.service_helper.get_features().await;

        if !features.contains(&"disable_recovery".to_string()) {
            // Register for recovery events
            info!("Setting up recovery for Twitch adapter");
            self.setup_recovery().await?;
        }

        // Check if already authenticated or try to restore from saved tokens
        match self.restore_authentication_if_available().await {
            Ok(true) => {
                // If we successfully restored auth, connect to EventSub
                info!("Successfully restored authentication, starting EventSub");
                self.start_eventsub().await?;

                // Start polling for stream/channel info
                self.start_periodic_polls().await?;
            }
            Ok(false) => {
                info!("No saved authentication found, waiting for user to authenticate");
                *self.pending_auth.write().await = true;
                // We'll connect after authentication completes
            }
            Err(e) => {
                error!("Error restoring authentication: {}", e);
                return Err(AdapterError::auth(format!(
                    "Error restoring authentication: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// Disconnect from the service
    async fn disconnect(&self) -> Result<(), AdapterError> {
        // Stop the polling loop first
        self.base.stop_polling().await?;

        // Record the disconnect operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "disconnect",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // Disconnect EventSub client
        if let Some(eventsub) = self.eventsub_client.read().await.as_ref() {
            if let Err(e) = eventsub.stop().await {
                warn!("Error stopping EventSub client: {}", e);
                // Non-critical error, continue with disconnect
            }
        }

        // Clear EventSub client
        *self.eventsub_client.write().await = None;

        // Reset all stored state
        *self.stream_info.write().await = None;
        *self.channel_info.write().await = None;
        *self.pending_auth.write().await = false;
        *self.last_stream_id.write().await = None;

        Ok(())
    }

    /// Get a feature by key from service helper
    async fn get_feature(&self, key: &str) -> Result<Option<String>, AdapterError> {
        self.service_helper.get_feature(key).await
    }

    /// Check the connection status
    async fn check_connection(&self) -> Result<bool, AdapterError> {
        // First check if authenticated
        if !self.auth_manager.is_authenticated().await {
            return Ok(false);
        }

        // Then check if EventSub is connected (if we have an EventSub client)
        if let Some(eventsub) = self.eventsub_client.read().await.as_ref() {
            return Ok(eventsub.is_connected());
        }

        // No EventSub client means we're not fully connected
        Ok(false)
    }

    /// Get the adapter status as JSON
    async fn get_status(&self) -> Result<Value, AdapterError> {
        // Start with basic status
        let mut status = json!({
            "adapter_type": self.adapter_type(),
            "name": self.base.name(),
            "id": self.base.id(),
            "is_connected": self.check_connection().await?,
            "is_authenticated": self.auth_manager.is_authenticated().await,
            "pending_auth": *self.pending_auth.read().await,
        });

        // Add config info (but redact tokens)
        let config = self.config.read().await;
        let config_json = json!({
            "channel_id": config.channel_id,
            "channel_login": config.channel_login,
            "has_access_token": config.access_token.is_some(),
            "has_refresh_token": config.refresh_token.is_some(),
            "poll_interval_ms": config.poll_interval_ms,
        });
        status["config"] = config_json;

        // Add stream info if available
        if let Some(stream) = self.stream_info.read().await.as_ref() {
            status["stream"] = json!({
                "id": stream.id,
                "user_id": stream.user_id,
                "user_login": stream.user_login,
                "user_name": stream.user_name,
                "game_id": stream.game_id,
                "game_name": stream.game_name,
                "title": stream.title,
                "viewer_count": stream.viewer_count,
                "started_at": stream.started_at,
                "is_mature": stream.is_mature,
            });
        } else {
            status["stream"] = json!(null);
        }

        // Add channel info if available
        if let Some(channel) = self.channel_info.read().await.as_ref() {
            status["channel"] = json!({
                "broadcaster_id": channel.broadcaster_id,
                "broadcaster_login": channel.broadcaster_login,
                "broadcaster_name": channel.broadcaster_name,
                "game_id": channel.game_id,
                "game_name": channel.game_name,
                "title": channel.title,
            });
        } else {
            status["channel"] = json!(null);
        }

        Ok(status)
    }

    /// Handle lifecycle events from the manager
    async fn handle_lifecycle_event(
        &self,
        event: &str,
        data: Option<&Value>,
    ) -> Result<(), AdapterError> {
        debug!("Handling lifecycle event: {}", event);

        match event {
            "initialize" => {
                // Already handled in the constructor
                Ok(())
            }
            "restore" => {
                if let Some(data) = data {
                    if let Some(token_data) = data.get("token_data") {
                        info!("Restoring token data from lifecycle event");

                        // Parse token data
                        let token_data: TokenData = serde_json::from_value(token_data.clone())
                            .map_err(|e| {
                                AdapterError::recovery(format!("Failed to parse token data: {}", e))
                            })?;

                        // Extract tokens
                        let access_token = token_data.access_token;
                        let refresh_token = token_data.refresh_token;

                        // Save tokens to config for future restores
                        let mut config = self.config.write().await;
                        config.access_token = Some(access_token.clone());
                        config.refresh_token = refresh_token.clone();

                        // Notify of config changes
                        drop(config);
                        self.base.save_config().await?;

                        // Restore authentication
                        self.auth_manager
                            .restore_from_saved_tokens(access_token, refresh_token)
                            .await
                            .map_err(|e| {
                                AdapterError::recovery(format!(
                                    "Failed to restore from saved tokens: {}",
                                    e
                                ))
                            })?;

                        // Start EventSub
                        self.start_eventsub().await?;

                        // Start polling
                        self.start_periodic_polls().await?;

                        // Reset pending auth flag
                        *self.pending_auth.write().await = false;
                    }
                }
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
        _args: Option<&Value>,
    ) -> Result<Value, AdapterError> {
        match command {
            "status" => self.get_status().await,
            "authenticate" => self
                .start_authentication()
                .await
                .map(|_| json!({"status": "authentication_started"})),
            "get_channel_info" => {
                let info = self.fetch_channel_info().await?;
                Ok(serde_json::to_value(info).unwrap_or_else(|_| json!(null)))
            }
            "get_stream_info" => {
                let info = self.fetch_stream_info().await?;
                Ok(serde_json::to_value(info).unwrap_or_else(|_| json!(null)))
            }
            "check_auth" => {
                let token = self.auth_manager.get_token().await;
                Ok(json!({
                    "is_authenticated": token.is_some(),
                    "token_expires_in": token.map(|t| t.expires_in().as_secs())
                }))
            }
            "poll_auth" => {
                self.poll_device_auth().await?;
                Ok(json!({"status": "success"}))
            }
            "disconnect" => {
                self.disconnect().await?;
                Ok(json!({"status": "disconnected"}))
            }
            _ => Err(AdapterError::invalid_command(format!(
                "Unknown command: {}",
                command
            ))),
        }
    }
}

/// Implementation for TwitchAdapter
impl TwitchAdapter {
    /// Create a new Twitch adapter
    pub fn new(
        name: &str,
        id: &str,
        event_bus: Arc<EventBus>,
        token_manager: Arc<TokenManager>,
        _recovery_manager: Arc<RecoveryManager>,
        config: Option<TwitchConfig>,
    ) -> Self {
        // Initialize with either provided config or default
        let config = config.unwrap_or_else(|| TwitchConfig {
            channel_id: None,
            channel_login: None,
            access_token: None,
            refresh_token: None,
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
        });

        // Get client ID from environment - this will be used for auth and API calls
        let client_id = match get_client_id() {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get Twitch client ID: {}", e);
                // Provide a default value for development, this will be properly validated on connect
                String::from("missing_client_id")
            }
        };

        // Create auth manager, API client, etc.
        let auth_manager = TwitchAuthManager::new(client_id.clone());
        let api_client = TwitchApiClient::new();

        // Create base adapter
        let base = BaseAdapter::new(
            name,
            id,
            event_bus.clone(),
            serde_json::to_value(config.clone()).unwrap_or_default(),
            token_manager.clone(),
        );

        // Create service helper
        let service_helper = ServiceHelperImpl::new(Arc::new(base.clone()));

        Self {
            base,
            auth_manager,
            api_client,
            eventsub_client: RwLock::new(None),
            stream_info: RwLock::new(None),
            channel_info: RwLock::new(None),
            pending_auth: RwLock::new(false),
            config: RwLock::new(config),
            last_stream_id: RwLock::new(None),
            service_helper,
        }
    }

    /// Get the adapter ID
    pub fn id(&self) -> String {
        self.base.id().to_string()
    }

    /// Get the adapter name
    pub fn name(&self) -> String {
        self.base.name().to_string()
    }

    /// Create a new adapter with the provided configuration
    pub fn with_config(
        name: &str,
        id: &str,
        event_bus: Arc<EventBus>,
        config: TwitchConfig,
    ) -> Self {
        Self::new(
            name,
            id,
            event_bus,
            Arc::new(TokenManager::new()),
            Arc::new(RecoveryManager::new()),
            Some(config),
        )
    }

    /// Start authentication process
    pub async fn start_authentication(&self) -> Result<(), AdapterError> {
        info!("Starting Twitch authentication process");

        // Record the operation
        TraceHelper::record_adapter_operation(
            self.adapter_type(),
            "start_authentication",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await;

        // Register an auth callback to store tokens
        self.auth_manager
            .register_auth_callback(move |event| {
                match event {
                    AuthEvent::AuthenticationSuccess => {
                        info!("Authentication successful");
                        // Tokens will be saved in the polling completion
                        Ok(())
                    }
                    AuthEvent::TokenRefreshDetails {
                        expires_in,
                        payload,
                    } => {
                        info!("Token refreshed, expires in {} seconds", expires_in);
                        // Token manager will handle token storage
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })
            .await
            .map_err(|e| AdapterError::auth(format!("Failed to register auth callback: {}", e)))?;

        // Start device auth flow
        let device_code = self
            .auth_manager
            .start_device_auth(vec![])
            .await
            .map_err(|e| AdapterError::auth(format!("Failed to start authentication: {}", e)))?;

        // Set pending auth flag
        *self.pending_auth.write().await = true;

        Ok(())
    }

    /// Poll for device authentication completion
    #[instrument(skip(self), fields(adapter_id = %self.id(), adapter_type = %self.adapter_type()))]
    pub async fn poll_device_auth(&self) -> Result<(), AdapterError> {
        debug!("Polling for device authentication completion");

        // Check if we're pending auth
        if !*self.pending_auth.read().await {
            debug!("Not pending authentication, skipping poll");
            return Ok(());
        }

        // Check if we're in device auth state
        if !self.auth_manager.is_pending_device_auth().await {
            debug!("Not in device auth state, skipping poll");
            return Ok(());
        }

        // Poll for completion
        match self.auth_manager.poll_device_auth().await {
            Ok(token) => {
                info!("Device authentication completed successfully");

                // Save auth tokens to config
                let (access_token, refresh_token) = (
                    token.access_token.secret().to_string(),
                    token.refresh_token.as_ref().map(|t| t.secret().to_string()),
                );

                // Update config
                let mut config = self.config.write().await;
                config.access_token = Some(access_token.clone());
                config.refresh_token = refresh_token.clone();
                drop(config);

                // Save config
                self.base.save_config().await?;

                // Reset pending auth flag
                *self.pending_auth.write().await = false;

                // Start EventSub client
                self.start_eventsub().await?;

                // Start polling for stream/channel info
                self.start_periodic_polls().await?;

                Ok(())
            }
            Err(e) => {
                // Check if we're still waiting
                if e.to_string().contains("authorization_pending") {
                    debug!("Device auth still pending, user has not completed auth yet");
                    return Ok(());
                }

                // Handle error
                error!("Device authentication failed: {}", e);

                // Reset pending auth flag
                *self.pending_auth.write().await = false;

                Err(AdapterError::auth(format!("Authentication failed: {}", e)))
            }
        }
    }

    /// Restore authentication from saved tokens if available
    async fn restore_authentication_if_available(&self) -> Result<bool, AdapterError> {
        let config = self.config.read().await;

        if let (Some(access_token), refresh_token) =
            (config.access_token.clone(), config.refresh_token.clone())
        {
            info!("Attempting to restore authentication from saved tokens");

            // Drop the lock on config before async operations
            drop(config);

            // Try to restore authentication
            match self
                .auth_manager
                .restore_from_saved_tokens(access_token, refresh_token)
                .await
            {
                Ok(_) => {
                    info!("Successfully restored authentication from saved tokens");
                    return Ok(true);
                }
                Err(e) => {
                    warn!("Failed to restore authentication: {}", e);

                    // Clear tokens from config
                    let mut config = self.config.write().await;
                    config.access_token = None;
                    config.refresh_token = None;
                    drop(config);

                    // Save updated config
                    self.base.save_config().await?;

                    // Return false to indicate we couldn't restore
                    return Ok(false);
                }
            }
        }

        Ok(false)
    }

    /// Start EventSub client
    async fn start_eventsub(&self) -> Result<(), AdapterError> {
        // Create EventSub client
        let eventsub = EventSubClient::new(self.base.event_bus()).await?;

        // Set token refresher callback
        let auth_manager = self.auth_manager.clone();
        eventsub.set_token_refresher(Arc::new(move || {
            let auth_manager = auth_manager.clone();
            Box::pin(async move {
                auth_manager.refresh_token_if_needed().await?;
                let token = auth_manager
                    .get_token()
                    .await
                    .ok_or_else(|| anyhow!("Token should be available after refresh but wasn't"))?;
                Ok(token)
            })
        }));

        // Get token
        let token = self
            .auth_manager
            .get_token()
            .await
            .ok_or_else(|| AdapterError::auth("No token available to start EventSub client"))?;

        // Start EventSub client
        eventsub.start(&token).await?;

        // Store EventSub client
        *self.eventsub_client.write().await = Some(eventsub);

        info!("Started EventSub client successfully");

        Ok(())
    }

    /// Register for recovery events
    async fn setup_recovery(&self) -> Result<(), AdapterError> {
        info!("Setting up recovery for Twitch adapter");

        // Create recovery data generator closure
        let auth_manager = self.auth_manager.clone();
        let recovery_data_generator = Box::new(move || {
            let auth_manager = auth_manager.clone();
            Box::pin(async move {
                match auth_manager.get_token_for_storage().await {
                    Some((access_token, refresh_token)) => {
                        // Create token data
                        let token_data = TokenData {
                            access_token,
                            refresh_token,
                            expires_in: None, // We don't store this as the token refreshes
                            metadata: HashMap::new(),
                        };

                        // Create recovery data
                        let mut recovery_data = serde_json::Map::new();
                        recovery_data.insert(
                            "token_data".to_string(),
                            serde_json::to_value(token_data).unwrap_or_default(),
                        );

                        Ok::<Option<serde_json::Value>, AdapterError>(Some(
                            serde_json::Value::Object(recovery_data),
                        ))
                    }
                    None => Ok(None),
                }
            })
        });

        // Register for recovery
        let base_id = self.base.id();
        // For now, skip recovery registration until we fix the AdapterRecovery trait
        // let recovery = AdapterRecovery::new(
        //     self.adapter_type(),
        //     &base_id,
        //     recovery_data_generator,
        // );

        // Access the recovery manager through the service helper and register
        // For now, skip the recovery registration
        // self.service_helper
        //     .register_for_recovery(recovery)
        //     .await
        //     .map_err(|e| {
        //         AdapterError::recovery(format!("Failed to register for recovery: {}", e))
        //     })?;

        Ok(())
    }

    /// Start periodic polling
    async fn start_periodic_polls(&self) -> Result<(), AdapterError> {
        info!("Starting periodic polls for Twitch adapter");

        // Get poll interval from config
        let poll_interval = {
            let config = self.config.read().await;
            Duration::from_millis(config.poll_interval_ms)
        };

        // Clone self reference for poll function
        let adapter = self.clone();

        // Create poll function
        let poll_fn = Box::new(move || {
            let adapter = adapter.clone();
            Box::pin(async move {
                // Skip polling if not authenticated
                if !adapter.auth_manager.is_authenticated().await {
                    debug!("Skipping poll - not authenticated");
                    return Ok(());
                }

                // Instead of using Box::pin which causes type issues,
                // execute each poll sequentially

                // Poll 1: Check token
                debug!("Checking Twitch token");
                let auth_manager = adapter.auth_manager.clone();
                match auth_manager.refresh_token_if_needed().await {
                    Ok(_) => debug!("Token check successful"),
                    Err(e) => warn!("Token check failed: {}", e),
                }

                // Poll 2: Stream info
                debug!("Fetching Twitch stream info");
                match adapter.fetch_stream_info().await {
                    Ok(_) => debug!("Stream info fetch successful"),
                    Err(e) => warn!("Stream info fetch failed: {}", e),
                }

                // Poll 3: Channel info
                debug!("Fetching Twitch channel info");
                match adapter.fetch_channel_info().await {
                    Ok(_) => debug!("Channel info fetch successful"),
                    Err(e) => warn!("Channel info fetch failed: {}", e),
                }

                Ok(())
            })
        });

        // Start polling
        self.base.start_polling(poll_fn, poll_interval).await?;

        info!("Twitch adapter polling started");

        Ok(())
    }

    /// Fetch stream information
    async fn fetch_stream_info(&self) -> Result<Option<Stream>, AdapterError> {
        // Check if authenticated
        if !self.auth_manager.is_authenticated().await {
            return Err(AdapterError::auth("Not authenticated"));
        }

        // Get token
        let token = self
            .auth_manager
            .get_token()
            .await
            .ok_or_else(|| AdapterError::auth("No token available"))?;

        // Get channel ID and login from config
        let (channel_id, channel_login) = {
            let config = self.config.read().await;
            extract_channel_info(&config)
        };

        // We need at least one of these
        if channel_id.is_none() && channel_login.is_none() {
            return Err(AdapterError::config(
                "Neither channel_id nor channel_login is configured",
            ));
        }

        // Fetch stream info
        let stream_info = self
            .api_client
            .fetch_stream_info(&token, channel_id.as_deref(), channel_login.as_deref())
            .await?;

        // Store the fetched info
        let mut stream_info_lock = self.stream_info.write().await;

        // Check if stream changed
        let stream_changed = match (&*stream_info_lock, &stream_info) {
            (Some(old), Some(new)) => old.id != new.id,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (None, None) => false,
        };

        // Get the last stream ID
        let old_stream_id = match &*stream_info_lock {
            Some(stream) => Some(stream.id.clone()),
            None => None,
        };

        // Update stored info
        *stream_info_lock = stream_info.clone();
        drop(stream_info_lock);

        // If stream changed, send events
        if stream_changed {
            if let Some(stream) = &stream_info {
                // Stream went online
                if old_stream_id.is_none() {
                    info!("Stream went online: {}", stream.id);

                    // Send stream.online event
                    self.base
                        .event_bus()
                        .publish(StreamEvent::new(
                            self.adapter_type(),
                            "stream.online",
                            serde_json::to_value(stream).unwrap_or_default(),
                        ))
                        .await?;
                } else {
                    // Stream changed (old stream ended, new stream started)
                    let old_id_str = match &old_stream_id {
                        Some(id) => id.to_string(),
                        None => "none".to_string(),
                    };
                    info!(
                        "Stream changed: {} -> {}",
                        old_id_str,
                        stream.id.to_string()
                    );

                    // Send stream.offline for old stream
                    if let Some(old_id) = old_stream_id {
                        self.base
                            .event_bus()
                            .publish(StreamEvent::new(
                                self.adapter_type(),
                                "stream.offline",
                                json!({ "id": old_id }),
                            ))
                            .await?;
                    }

                    // Send stream.online for new stream
                    self.base
                        .event_bus()
                        .publish(StreamEvent::new(
                            self.adapter_type(),
                            "stream.online",
                            serde_json::to_value(stream).unwrap_or_default(),
                        ))
                        .await?;
                }

                // Update last stream ID
                *self.last_stream_id.write().await = Some(stream.id.to_string());
            } else if old_stream_id.is_some() {
                // Stream went offline
                let old_id_str = match &old_stream_id {
                    Some(id) => id.to_string(),
                    None => "none".to_string(),
                };
                info!("Stream went offline: {}", old_id_str);

                // Send stream.offline event
                self.base
                    .event_bus()
                    .publish(StreamEvent::new(
                        self.adapter_type(),
                        "stream.offline",
                        json!({ "id": old_id_str }),
                    ))
                    .await?;

                // Clear last stream ID
                *self.last_stream_id.write().await = None;
            }
        }

        Ok(stream_info)
    }

    /// Fetch channel information
    async fn fetch_channel_info(&self) -> Result<Option<ChannelInformation>, AdapterError> {
        // Check if authenticated
        if !self.auth_manager.is_authenticated().await {
            return Err(AdapterError::auth("Not authenticated"));
        }

        // Get token
        let token = self
            .auth_manager
            .get_token()
            .await
            .ok_or_else(|| AdapterError::auth("No token available"))?;

        // Get channel ID from config
        let (channel_id, _) = {
            let config = self.config.read().await;
            extract_channel_info(&config)
        };

        // We need a channel ID for this call
        let broadcaster_id = match channel_id {
            Some(id) => id,
            None => {
                // Try to get it from stream info
                if let Some(stream) = self.stream_info.read().await.as_ref() {
                    stream.user_id.to_string()
                } else {
                    return Err(AdapterError::config(
                        "No channel_id configured or available from stream info",
                    ));
                }
            }
        };

        // Fetch channel info
        let channel_info = self
            .api_client
            .fetch_channel_info(&token, &broadcaster_id)
            .await?;

        // Store the fetched info
        let mut channel_info_lock = self.channel_info.write().await;

        // Check if channel info changed
        let channel_changed = match (&*channel_info_lock, &channel_info) {
            (Some(old), Some(new)) => {
                old.title != new.title
                    || old.game_id != new.game_id
                    || old.game_name != new.game_name
            }
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (None, None) => false,
        };

        // Update stored info
        *channel_info_lock = channel_info.clone();
        drop(channel_info_lock);

        // If channel changed, send event
        if channel_changed {
            if let Some(channel) = &channel_info {
                info!("Channel info changed: {}", channel.broadcaster_name);

                // Send channel.updated event
                self.base
                    .event_bus()
                    .publish(StreamEvent::new(
                        self.adapter_type(),
                        "channel.updated",
                        serde_json::to_value(channel).unwrap_or_default(),
                    ))
                    .await?;
            }
        }

        Ok(channel_info)
    }
}

// Implement Clone for TwitchAdapter
impl Clone for TwitchAdapter {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            auth_manager: self.auth_manager.clone(),
            api_client: self.api_client.clone(),
            eventsub_client: RwLock::new(self.eventsub_client.blocking_read().clone()),
            stream_info: RwLock::new(self.stream_info.blocking_read().clone()),
            channel_info: RwLock::new(self.channel_info.blocking_read().clone()),
            pending_auth: RwLock::new(*self.pending_auth.blocking_read()),
            config: RwLock::new(self.config.blocking_read().clone()),
            last_stream_id: RwLock::new(self.last_stream_id.blocking_read().clone()),
            service_helper: self.service_helper.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    pub use crate::adapters::tests::twitch_test::*;
}
