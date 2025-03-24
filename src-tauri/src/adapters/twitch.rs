use crate::{
    adapters::base::{AdapterConfig, BaseAdapter},
    adapters::twitch_eventsub::EventSubClient,
    auth::token_manager::{TokenData, TokenManager},
    recovery::{AdapterRecovery, RecoveryManager},
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, sync::Arc};
use tauri::async_runtime::RwLock;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::TwitchToken;

use super::twitch_api::TwitchApiClient;
use super::twitch_auth::{AuthEvent, TwitchAuthManager};

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
    pub poll_interval_ms: u64,
    /// Whether to monitor channel info
    pub monitor_channel_info: bool,
    /// Whether to monitor stream status
    pub monitor_stream_status: bool,
    /// Whether to use EventSub instead of polling
    pub use_eventsub: bool,
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
            use_eventsub: true, // Default to using EventSub
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
            "use_eventsub": self.use_eventsub,
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

        // Extract EventSub flag
        if let Some(use_eventsub) = json.get("use_eventsub").and_then(|v| v.as_bool()) {
            config.use_eventsub = use_eventsub;
        }

        Ok(config)
    }

    fn adapter_type() -> &'static str {
        "twitch"
    }

    fn validate(&self) -> Result<()> {
        // Need either channel ID or channel login
        if self.channel_id.is_none() && self.channel_login.is_none() {
            return Err(anyhow!(
                "Either Channel ID or Channel Login must be provided"
            ));
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
    /// Token manager for centralized auth handling
    token_manager: Option<Arc<TokenManager>>,
    /// EventSub client for WebSocket-based events
    eventsub_client: Arc<RwLock<Option<EventSubClient>>>,
    /// Recovery manager for error handling and retries
    recovery_manager: Option<Arc<RecoveryManager>>,
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
            }
            Err(e) => {
                error!("Failed to get Twitch Client ID from environment: {}", e);
                String::new()
            }
        };

        let config = TwitchConfig::default();

        Self {
            base: BaseAdapter::new("twitch", event_bus.clone()),
            config: Arc::new(RwLock::new(config.clone())),
            auth_manager: Arc::new(RwLock::new(TwitchAuthManager::new(client_id))),
            api_client: TwitchApiClient::new(),
            state: Arc::new(RwLock::new(TwitchState::default())),
            token_manager: None, // Will be set when connected to StreamService
            eventsub_client: Arc::new(RwLock::new(None)),
            recovery_manager: None, // Will be set when connected to StreamService
        }
    }

    /// Set the token manager
    pub fn set_token_manager(&mut self, token_manager: Arc<TokenManager>) {
        self.token_manager = Some(token_manager);
    }

    /// Set the recovery manager
    pub fn set_recovery_manager(&mut self, recovery_manager: Arc<RecoveryManager>) {
        self.recovery_manager = Some(recovery_manager);
    }

    /// Attempt to load tokens from the TokenManager
    #[instrument(skip(self), level = "debug")]
    pub async fn load_tokens_from_manager(&self) -> Result<bool> {
        if let Some(tm) = &self.token_manager {
            info!("Attempting to load Twitch tokens from TokenManager");
            
            // First ensure token expiration is set
            if let Err(e) = tm.ensure_twitch_token_expiration().await {
                warn!("Failed to ensure token expiration during loading: {}", e);
            }

            match tm.get_tokens("twitch").await {
                Ok(Some(token_data)) => {
                    info!("Found tokens in TokenManager");

                    // Update adapter config with tokens
                    let mut config = self.config.write().await;
                    config.access_token = Some(token_data.access_token.clone());
                    config.refresh_token = token_data.refresh_token.clone();

                    // Log information about the token's expiration
                    if token_data.is_expired() {
                        warn!("Retrieved tokens are expired, may need refresh");
                    } else if let Some(expires_in) = token_data.expires_in {
                        let now = chrono::Utc::now();
                        let duration = expires_in.signed_duration_since(now);
                        info!("Token expires in {} seconds", duration.num_seconds());
                    }
                    
                    // Check for refresh token age - DCF tokens have a 30-day limit
                    if let Some(created_str) = token_data.get_metadata_value("refresh_token_created_at") {
                        if let Some(created_str) = created_str.as_str() {
                            if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_str) {
                                let created_utc = created.with_timezone(&chrono::Utc);
                                let now = chrono::Utc::now();
                                let age = now - created_utc;
                                let days_remaining = 30 - age.num_days();
                                
                                if days_remaining <= 0 {
                                    warn!("Refresh token is potentially expired - it was created {} days ago (30-day limit)", age.num_days());
                                } else if days_remaining < 5 {
                                    warn!("Refresh token is approaching expiry - only {} days remaining before 30-day limit", days_remaining);
                                } else {
                                    info!("Refresh token has {} days remaining before 30-day limit", days_remaining);
                                }
                            }
                        }
                    } else {
                        // If we don't have a creation timestamp, that's concerning
                        warn!("No refresh token creation timestamp found - unable to check 30-day expiry status");
                    }

                    // Try to restore auth state
                    if let Err(e) = self.restore_token_auth_state().await {
                        warn!(
                            "Failed to restore auth state from TokenManager tokens: {}",
                            e
                        );
                        return Ok(false);
                    }

                    return Ok(true);
                }
                Ok(None) => {
                    debug!("No Twitch tokens found in TokenManager");
                }
                Err(e) => {
                    error!("Error retrieving tokens from TokenManager: {}", e);
                }
            }
        } else {
            debug!("TokenManager not available, can't load tokens");
        }

        Ok(false)
    }

    /// Restore authentication state from config tokens
    async fn restore_token_auth_state(&self) -> Result<()> {
        let config = self.config.read().await.clone();

        if let (Some(access_token), refresh_token) = (&config.access_token, &config.refresh_token) {
            info!("Restoring authentication from tokens in config");

            match self
                .auth_manager
                .write()
                .await
                .restore_from_saved_tokens(access_token.clone(), refresh_token.clone())
                .await
            {
                Ok(token) => {
                    info!("Successfully restored authentication state");
                    
                    // Now ensure token details are stored properly in TokenManager
                    if let Some(tm) = &self.token_manager {
                        // Extract tokens
                        let access_token = token.access_token.secret().to_string();
                        let refresh_token = token.refresh_token.as_ref().map(|t| t.secret().to_string());
                        
                        // Create TokenData with full expiration information
                        let mut token_data = TokenData::new(access_token, refresh_token);
                        
                        // Set expiration from token
                        let expires_in = token.expires_in().as_secs();
                        
                        // Default to 4 hours (14400 seconds) if no expiration is provided
                        let expires_to_use = if expires_in > 0 {
                            info!("Setting token expiration to {} seconds ({} minutes) from now", 
                                 expires_in, expires_in / 60);
                            expires_in
                        } else {
                            warn!("Token has invalid expires_in value: {}. Defaulting to 4 hours (14400 seconds)", expires_in);
                            14400 // Default to 4 hours, which is the standard Twitch token lifetime
                        };
                        
                        // Always set an expiration, even if we have to use the default
                        token_data.set_expiration(expires_to_use);
                        
                        // Track when the refresh token was created/refreshed
                        token_data.track_refresh_token_created();
                        
                        // Store in TokenManager
                        match tm.store_tokens("twitch", token_data).await {
                            Ok(_) => {
                                info!("Successfully updated token data in TokenManager with expiration info");
                                
                                // Double-check that expiration is set (handles null expiration case)
                                if let Err(e) = tm.ensure_twitch_token_expiration().await {
                                    warn!("Failed to ensure token expiration: {}", e);
                                }
                            },
                            Err(e) => warn!("Failed to update token data in TokenManager: {}", e),
                        }
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to restore authentication state: {}", e);
                    Err(anyhow!("Failed to restore auth state: {}", e))
                }
            }
        } else {
            Err(anyhow!("No tokens available in config"))
        }
    }

    /// Initialize the EventSub client
    #[instrument(skip(self), level = "debug")]
    async fn init_eventsub_client(&self) -> Result<()> {
        info!("Initializing EventSub client");

        // Create new EventSub client
        let event_bus = self.base.event_bus();
        match EventSubClient::new(event_bus.clone()) {
            Ok(client) => {
                // Store client in adapter
                *self.eventsub_client.write().await = Some(client);
                info!("EventSub client initialized");
                Ok(())
            }
            Err(e) => {
                error!("Failed to create EventSub client: {}", e);
                // Fall back to polling by setting use_eventsub to false
                let mut config = self.config.write().await;
                config.use_eventsub = false;
                Err(anyhow!("Failed to create EventSub client: {}", e))
            }
        }
    }

    /// Start the EventSub client with the current token
    #[instrument(skip(self), level = "debug")]
    async fn start_eventsub(&self) -> Result<()> {
        info!("Starting EventSub client");

        // Check config for token and EventSub setting
        let config = self.config.read().await;
        info!("Config settings: has_access_token={}, use_eventsub={}", 
              config.access_token.is_some(), config.use_eventsub);
              
        if !config.use_eventsub {
            info!("EventSub is disabled in config, enabling it");
            drop(config);
            let mut config = self.config.write().await;
            config.use_eventsub = true;
            drop(config);
        } else {
            drop(config);
        }

        // Get access token with detailed logging
        let token = {
            info!("Attempting to get token from auth manager");
            let auth_manager = self.auth_manager.read().await;
            match auth_manager.get_token().await {
                Some(token) => {
                    info!("Auth manager has valid token: access_token_len={}, has_refresh_token={}, expires_in={}s", 
                          token.access_token.secret().len(),
                          token.refresh_token.is_some(),
                          token.expires_in().as_secs());
                    drop(auth_manager);
                    token
                }
                None => {
                    warn!("Auth manager has no token, attempting to restore from config");
                    // Try to restore from config
                    drop(auth_manager);
                    match self.restore_token_auth_state().await {
                        Ok(_) => {
                            info!("Successfully restored token from config");
                        }
                        Err(e) => {
                            warn!("Failed to restore auth state from config: {}", e);
                        }
                    }

                    // Try again
                    let auth_manager = self.auth_manager.read().await;
                    match auth_manager.get_token().await {
                        Some(token) => {
                            info!("Successfully retrieved token after restoration: access_token_len={}, has_refresh_token={}", 
                                  token.access_token.secret().len(),
                                  token.refresh_token.is_some());
                            drop(auth_manager);
                            token
                        }
                        None => {
                            error!("No access token available after restoration attempt");
                            return Err(anyhow!(
                                "No access token available after restoration attempt"
                            ));
                        }
                    }
                }
            }
        };

        // Check if the token has a user ID
        let user_id = token.user_id.clone();
        info!("Token has user ID: {}", user_id);

        // If no EventSub client exists, initialize it
        if self.eventsub_client.read().await.is_none() {
            info!("No EventSub client exists, initializing one");
            match self.init_eventsub_client().await {
                Ok(_) => info!("Successfully initialized EventSub client"),
                Err(e) => {
                    error!("Failed to initialize EventSub client: {}", e);
                    return Err(anyhow!("Failed to initialize EventSub client: {}", e));
                }
            }
        } else {
            info!("EventSub client already exists");
        }

        // Start EventSub client
        let eventsub_client = self.eventsub_client.read().await;
        if let Some(client) = &*eventsub_client {
            info!("Setting up token refresher callback for EventSub client");
            
            // Set up token refresher callback before starting
            let auth_manager = Arc::clone(&self.auth_manager);
            let token_manager = self.token_manager.clone();
            
            // Create token refresher callback
            let refresher: crate::adapters::twitch_eventsub::TokenRefresher = Arc::new(move || {
                let auth_manager_clone = auth_manager.clone();
                let token_manager_clone = token_manager.clone();
                
                Box::pin(async move {
                    info!("Token refresher called - checking if token needs refresh");
                    
                    // First try to refresh using auth manager
                    let refresh_successful = match auth_manager_clone.write().await.refresh_token_if_needed().await {
                        Ok(_) => {
                            info!("Token refresh check completed successfully");
                            true
                        },
                        Err(e) => {
                            warn!("Token refresh attempt failed: {}", e);
                            false
                        }
                    };
                    
                    // Regardless of refresh result, get the current token
                    match auth_manager_clone.read().await.get_token().await {
                        Some(token) => {
                            info!("Token refresher returning valid token: expires_in={}s", 
                                  token.expires_in().as_secs());
                            
                            // If refresh succeeded, update token in TokenManager
                            if refresh_successful && token_manager_clone.is_some() {
                                // Extract tokens
                                let access_token = token.access_token.secret().to_string();
                                let refresh_token = token.refresh_token.as_ref().map(|t| t.secret().to_string());
                                
                                // Create TokenData
                                let mut token_data = crate::auth::token_manager::TokenData::new(
                                    access_token, refresh_token
                                );
                                
                                // Set expiration
                                let expires_in = token.expires_in().as_secs();
                                token_data.set_expiration(expires_in);
                                
                                // Store in token manager
                                if let Some(tm) = &token_manager_clone {
                                    match tm.store_tokens("twitch", token_data).await {
                                        Ok(_) => {
                                            info!("Successfully updated token data in TokenManager, ensuring persistence");
                                            
                                            // The most important part is ensuring the token is written to secure storage
                                            // This happens automatically when store_tokens is called on TokenManager
                                            
                                            info!("Refreshed token has been persisted to secure storage");
                                            
                                            // Log that we're ensuring this token will survive restarts
                                            info!("Token refresh complete - refreshed tokens will be used after restart");
                                            
                                            // Log expiration details for debugging
                                            info!("Access token expires in: {}s, refresh token was present: {}", 
                                                 token.expires_in().as_secs(), token.refresh_token.is_some());
                                        },
                                        Err(e) => warn!("Failed to update token data in TokenManager: {}", e),
                                    }
                                }
                            }
                            
                            Ok(token.clone())
                        },
                        None => {
                            error!("No token available for refresh in token refresher");
                            Err(anyhow!("No token available for refresh"))
                        }
                    }
                })
            });
            
            // Set the token refresher on the EventSub client
            info!("Setting token refresher on EventSub client");
            client.set_token_refresher(refresher);
            
            // Now start the client with the token
            info!("Attempting to start EventSub client with token");
            match client.start(&token).await {
                Ok(_) => {
                    info!("EventSub client started successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to start EventSub client: {}", e);
                    // Fall back to polling
                    let mut config = self.config.write().await;
                    config.use_eventsub = false;
                    Err(anyhow!("Failed to start EventSub client: {}", e))
                }
            }
        } else {
            error!("EventSub client not initialized despite initialization attempt");
            // Fall back to polling
            let mut config = self.config.write().await;
            config.use_eventsub = false;
            Err(anyhow!("EventSub client not initialized"))
        }
    }

    /// Stop the EventSub client
    #[instrument(skip(self), level = "debug")]
    async fn stop_eventsub(&self) -> Result<()> {
        info!("Stopping EventSub client");

        if let Some(client) = &*self.eventsub_client.read().await {
            if let Err(e) = client.stop().await {
                warn!("Error stopping EventSub client: {}", e);
                // Continue anyway, the client will be released
            }
            info!("EventSub client stopped");
        }

        Ok(())
    }

    /// Create a new Twitch adapter with custom config
    #[instrument(skip(event_bus), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: TwitchConfig) -> Self {
        info!(
            channel_id = ?config.channel_id,
            channel_login = ?config.channel_login,
            use_eventsub = config.use_eventsub,
            "Creating Twitch adapter with custom config"
        );

        // Get the client ID from environment
        let client_id = match get_client_id() {
            Ok(id) => {
                info!("Using Twitch Client ID from environment variable");
                id
            }
            Err(e) => {
                error!("Failed to get Twitch Client ID from environment: {}", e);
                String::new()
            }
        };

        // Create adapter instance - we'll set the auth callback separately
        let adapter = Self {
            base: BaseAdapter::new("twitch", event_bus.clone()),
            config: Arc::new(RwLock::new(config.clone())),
            auth_manager: Arc::new(RwLock::new(TwitchAuthManager::new(client_id))),
            api_client: TwitchApiClient::new(),
            state: Arc::new(RwLock::new(TwitchState::default())),
            token_manager: None, // Will be set when connected to service
            eventsub_client: Arc::new(RwLock::new(None)),
            recovery_manager: None, // Will be set when connected to service
        };

        // Set up auth callback
        let event_bus_clone = event_bus.clone();
        let adapter_clone = adapter.clone();

        // Spawn a task to set up the auth callback asynchronously
        tauri::async_runtime::spawn(async move {
            // Make sure the auth_manager write lock is dropped before we use adapter_clone in the callback
            let setup_result = {
                let auth_manager_result = adapter_clone.auth_manager.try_write();
                if let Ok(mut auth_manager) = auth_manager_result {
                    // Create a clone specifically for the callback to avoid borrowing adapter_clone
                    let adapter_for_callback = adapter_clone.clone();
                    let event_bus_for_callback = event_bus_clone.clone();
                    
                    auth_manager.set_auth_callback(move |event| -> Result<()> {
                        let event_type = match &event {
                            AuthEvent::DeviceCodeReceived { .. } => "device_code",
                            AuthEvent::AuthenticationSuccess => "success",
                            AuthEvent::AuthenticationFailed { .. } => "failed",
                            AuthEvent::TokenRefreshed => "token_refreshed",
                            AuthEvent::TokenRefreshDetails { .. } => "token_refresh_details",
                            AuthEvent::TokenExpired { .. } => "token_expired",
                        };

                        let payload = match event {
                            AuthEvent::DeviceCodeReceived {
                                verification_uri,
                                user_code,
                                expires_in,
                            } => {
                                json!({
                                    "event": "device_code_received",
                                    "verification_uri": verification_uri,
                                    "user_code": user_code,
                                    "expires_in": expires_in,
                                    "message": format!("Please visit {} and enter code: {}",
                                                      verification_uri,
                                                      user_code),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })
                            }
                            AuthEvent::AuthenticationSuccess => {
                                // When authentication succeeds, trigger EventSub activation
                                // Clone for the task
                                let adapter_for_eventsub = adapter_for_callback.clone();
                                
                                // We cannot await here directly, so spawn a task to handle activation
                                tauri::async_runtime::spawn(async move {
                                    info!("Authentication success detected in callback - activating EventSub reactively");
                                    
                                    // First make sure EventSub is enabled in config
                                    {
                                        let mut config = adapter_for_eventsub.config.write().await;
                                        if !config.use_eventsub {
                                            info!("Enabling EventSub in config (was disabled)");
                                            config.use_eventsub = true;
                                        }
                                    }
                                    
                                    // Attempt to start EventSub
                                    match adapter_for_eventsub.start_eventsub().await {
                                        Ok(_) => info!("Successfully activated EventSub reactively after authentication"),
                                        Err(e) => warn!("Failed to activate EventSub reactively after authentication: {}", e)
                                    }
                                });
                                
                                json!({
                                    "event": "authentication_success",
                                    "message": "Successfully authenticated with Twitch",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })
                            }
                            AuthEvent::AuthenticationFailed { error } => {
                                json!({
                                    "event": "authentication_failed",
                                    "error": error,
                                    "message": "Failed to authenticate with Twitch",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })
                            }
                            AuthEvent::TokenRefreshed => {
                                // When token is refreshed, ensure EventSub is active if needed
                                // Clone for the task
                                let adapter_for_eventsub = adapter_for_callback.clone();
                                
                                // Spawn a task to check and activate if needed
                                tauri::async_runtime::spawn(async move {
                                    // Check if we need to (re)start EventSub
                                    let is_connected = adapter_for_eventsub.base.is_connected();
                                    let use_eventsub = adapter_for_eventsub.config.read().await.use_eventsub;
                                    
                                    if is_connected && use_eventsub {
                                        // Check current client state
                                        let client_status = adapter_for_eventsub.eventsub_client.read().await
                                            .as_ref()
                                            .map(|client| client.is_connected());
                                        
                                        // If no client or client isn't connected, activate
                                        if client_status.unwrap_or(false) == false {
                                            info!("Token refreshed and EventSub not active - activating EventSub reactively");
                                            match adapter_for_eventsub.start_eventsub().await {
                                                Ok(_) => info!("Successfully activated EventSub after token refresh"),
                                                Err(e) => warn!("Failed to activate EventSub after token refresh: {}", e)
                                            }
                                        }
                                    }
                                });
                                
                                json!({
                                    "event": "token_refreshed",
                                    "message": "Successfully refreshed Twitch token",
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })
                            }
                            AuthEvent::TokenRefreshDetails { expires_in: _, payload } => {
                                // Forward the payload from the auth manager
                                payload.clone()
                            }
                            AuthEvent::TokenExpired { error } => {
                                json!({
                                    "event": "token_expired",
                                    "message": "Token refresh failed, need to re-authenticate",
                                    "error": error,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })
                            }
                        };

                        // Since we can't use await in this callback, we'll spawn a task to publish
                        let event_type_str = format!("auth.{}", event_type);
                        let event = StreamEvent::new("twitch", &event_type_str, payload);

                        // Spawn a task to publish the event
                        let event_bus = event_bus_for_callback.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = event_bus.publish(event).await {
                                error!("Failed to publish event: {}", e);
                            }
                        });

                        Ok(())
                    });
                    Ok(())
                } else {
                    Err(anyhow!("Failed to acquire auth manager write lock"))
                }
            };
            
            // Log any errors from the setup
            if let Err(e) = setup_result {
                error!("Failed to set up auth callback: {}", e);
            }
        });

        // Restore from saved tokens if available
        if let (Some(access_token), refresh_token) = (&config.access_token, &config.refresh_token) {
            info!("Attempting to restore authentication from saved tokens");

            // Try to restore in a non-blocking way (spawn a task)
            let adapter_clone = adapter.clone();
            let access_token_clone = access_token.clone();
            let refresh_token_clone = refresh_token.clone();
            tauri::async_runtime::spawn(async move {
                let result = adapter_clone
                    .auth_manager
                    .write()
                    .await
                    .restore_from_saved_tokens(access_token_clone, refresh_token_clone)
                    .await;

                match result {
                    Ok(_) => {
                        info!("Successfully restored and validated authentication tokens");
                        
                        // Enable EventSub in config
                        {
                            let mut config = adapter_clone.config.write().await;
                            if !config.use_eventsub {
                                info!("Enabling EventSub in config (was disabled)");
                                config.use_eventsub = true;
                            }
                        }
                        
                        // Publish a token refresh event to trigger the auth callback's reactive behavior
                        info!("Publishing token refresh event to activate EventSub after token restoration");
                        
                        // Trigger the TokenRefreshed event through the auth manager
                        // This will cause the auth callback to activate EventSub reactively
                        match adapter_clone.auth_manager.read().await.trigger_auth_event(AuthEvent::TokenRefreshed).await {
                            Ok(_) => info!("Successfully triggered auth event for reactive EventSub activation"),
                            Err(e) => {
                                warn!("Failed to trigger auth event after token restoration: {}", e);
                                
                                // Fall back to direct activation as a safety measure
                                info!("Falling back to direct EventSub activation");
                                match adapter_clone.start_eventsub().await {
                                    Ok(_) => info!("Successfully started EventSub after token restoration (fallback)"),
                                    Err(err) => warn!("Failed to activate EventSub after token restoration: {}", err)
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Token validation failed, we need to notify the user
                        warn!(
                            "Failed to restore from saved tokens (invalid or expired): {}",
                            e
                        );

                        // Clear tokens from config since they're invalid
                        let mut config = adapter_clone.config.write().await;
                        config.access_token = None;
                        config.refresh_token = None;
                        
                        // Also clear tokens from TokenManager if available
                        if let Some(tm) = &adapter_clone.token_manager {
                            if let Err(remove_err) = tm.remove_tokens("twitch").await {
                                error!("Failed to remove invalid tokens from TokenManager: {}", remove_err);
                            } else {
                                info!("Successfully removed invalid tokens from TokenManager");
                            }
                        }

                        // Publish an event so the UI can notify the user
                        let payload = json!({
                            "event": "token_invalid",
                            "message": "Saved authentication tokens are invalid or expired. Please re-authenticate.",
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        // Publish via base adapter
                        if let Err(event_err) = adapter_clone
                            .base
                            .publish_event("auth.token_invalid", payload)
                            .await
                        {
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
        // Check if we're already in a pending device code flow
        // Result is captured but only used for side effects in the match arms
        let _is_pending = match self
            .auth_manager
            .read()
            .await
            .is_pending_device_auth()
            .await
        {
            true => {
                warn!("Abandoning previous device code authentication attempt");
                true
            }
            false => false,
        };

        // Get client ID from environment
        let client_id = match get_client_id() {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to get Twitch client ID: {}", e);
                return Err(e);
            }
        };

        info!("Starting Twitch authentication using device code flow");

        // Create a clone that shares the auth manager thanks to our improved Clone implementation
        let self_clone = self.clone();

        // Start the authentication process in a background task to avoid blocking
        tauri::async_runtime::spawn(async move {
            // Create HTTP client for requests
            let http_client = match reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
            {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to create HTTP client for Twitch auth: {}", e);
                    return;
                }
            };

            // Create the auth manager to access scopes
            let scopes = match self_clone.auth_manager.read().await.get_scopes() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to get Twitch scopes: {}", e);
                    return;
                }
            };

            // Create a new builder with the client ID and scopes
            let mut builder = twitch_oauth2::DeviceUserTokenBuilder::new(
                twitch_oauth2::ClientId::new(client_id.clone()),
                scopes.clone(),
            );

            // Start the device code flow
            let device_code = match builder.start(&http_client).await {
                Ok(code) => code,
                Err(e) => {
                    error!("Failed to start device code flow: {}", e);
                    return;
                }
            };

            // Store the device code in the auth manager
            if let Err(e) = self_clone
                .auth_manager
                .write()
                .await
                .set_device_code(device_code.clone())
            {
                error!("Failed to store device code: {}", e);
                return;
            }

            // Log the verification URI and code
            info!("=== TWITCH AUTHENTICATION REQUIRED ===");
            info!("Visit: {}", device_code.verification_uri);
            info!("Enter code: {}", device_code.user_code);
            info!("Code expires in {} seconds", device_code.expires_in);
            info!("======================================");

            // Publish event with the verification URI and code
            let event_payload = json!({
                "event": "device_code_received",
                "verification_uri": device_code.verification_uri,
                "user_code": device_code.user_code,
                "expires_in": device_code.expires_in,
                "message": format!("Please visit {} and enter code: {}",
                                  device_code.verification_uri,
                                  device_code.user_code),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) = self_clone
                .base
                .publish_event("auth.device_code", event_payload)
                .await
            {
                error!("Failed to publish device code event: {}", e);
            }

            // Poll for the token
            let start_time = std::time::Instant::now();
            let max_duration = std::time::Duration::from_secs(DEVICE_CODE_MAX_POLL_DURATION_SECS);

            loop {
                // Check if we've exceeded the maximum polling duration
                if start_time.elapsed() > max_duration {
                    error!(
                        "Device code polling timed out after {} seconds",
                        DEVICE_CODE_MAX_POLL_DURATION_SECS
                    );

                    // Reset auth state
                    if let Err(e) = self_clone.auth_manager.write().await.reset_auth_state() {
                        error!("Failed to reset auth state: {}", e);
                    }

                    // Publish timeout event
                    let event_payload = json!({
                        "event": "authentication_failed",
                        "error": "Authentication timed out",
                        "message": "Twitch authentication timed out",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });

                    if let Err(e) = self_clone
                        .base
                        .publish_event("auth.failed", event_payload)
                        .await
                    {
                        error!("Failed to publish timeout event: {}", e);
                    }

                    return;
                }

                // Poll for token status (with extra logging)
                info!("Polling Twitch for token status...");
                match builder.try_finish(&http_client).await {
                    Ok(token) => {
                        info!("Successfully authenticated with Twitch!");

                        // Store the token in the auth manager
                        let mut auth_manager = self_clone.auth_manager.write().await;
                        if let Err(e) = auth_manager.set_token(token.clone()) {
                            error!("Failed to store token: {}", e);
                            return;
                        }
                        
                        // Explicitly trigger AuthenticationSuccess event to activate EventSub reactively
                        if let Err(e) = auth_manager.trigger_auth_event(AuthEvent::AuthenticationSuccess).await {
                            warn!("Failed to trigger auth success event: {}", e);
                            // Continue anyway - the token is stored and we can still use it
                        }
                        
                        // Release the lock
                        drop(auth_manager);

                        // Extract tokens for config
                        let access_token = token.access_token.secret().to_string();
                        let refresh_token =
                            token.refresh_token.as_ref().map(|t| t.secret().to_string());

                        // Store tokens in TokenManager if available
                        if let Some(tm) = &self_clone.token_manager {
                            info!("Using TokenManager to store Twitch tokens");

                            // Create TokenData
                            let mut token_data =
                                TokenData::new(access_token.clone(), refresh_token.clone());

                            // Set expiration if available
                            let expires_in = token.expires_in().as_secs();
                            info!("Setting token expiration time in TokenData: {} seconds ({} minutes)", 
                                  expires_in, expires_in / 60);
                            
                            // Default to 4 hours (14400 seconds) if no expiration is provided
                            let expires_to_use = if expires_in > 0 {
                                expires_in
                            } else {
                                info!("Token has no expiration time, defaulting to 4 hours (14400 seconds)");
                                14400 // Default to 4 hours, which is the standard Twitch token lifetime
                            };
                            
                            // Always set an expiration, even if we have to use the default
                            token_data.set_expiration(expires_to_use);

                            // Track when the refresh token was created for 30-day monitoring
                            token_data.track_refresh_token_created();
                            token_data.set_metadata_value(
                                "auth_method",
                                serde_json::Value::String("device_code".to_string()),
                            );

                            info!(
                                "Set refresh token creation timestamp for 30-day expiry monitoring"
                            );

                            // Store in token manager
                            match tm.store_tokens("twitch", token_data).await {
                                Ok(_) => {
                                    info!("Successfully stored Twitch tokens in TokenManager");
                                    
                                    // Ensure expiration is set (handles null expiration case)
                                    if let Err(e) = tm.ensure_twitch_token_expiration().await {
                                        warn!("Failed to ensure token expiration: {}", e);
                                    } else {
                                        info!("Successfully ensured token expiration is set");
                                    }
                                },
                                Err(e) => error!("Failed to store tokens in TokenManager: {}", e),
                            }
                        } else {
                            warn!("TokenManager not available, storing tokens only in config");
                        }

                        // Update the config
                        let mut config = self_clone.config.write().await;
                        config.access_token = Some(access_token.clone());
                        config.refresh_token = refresh_token.clone();

                        // Log that we've saved tokens
                        info!(
                            "Saved authentication tokens to adapter config. access_token_len={}, refresh_token={}",
                            access_token.len(),
                            refresh_token.is_some()
                        );

                        // Force the tokens to be included in adapter settings
                        info!("Proactively updating adapter settings to include token information");

                        // Get a clone of the event bus for triggering an update
                        let event_bus = self_clone.base.event_bus();
                        // NOTE: Adapter name was previously unused - removed

                        // Clone the tokens for use in the task
                        let access_token_clone = access_token.clone();
                        let refresh_token_clone = refresh_token.clone();

                        // Spawn a task to wait a moment and then update settings
                        tauri::async_runtime::spawn(async move {
                            // Wait a moment for things to settle
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                            // Get the current adapter settings directly from the stream service
                            // Since we can't easily use the direct API here, we'll publish an event
                            // that asks the service to save its config with our tokens
                            let save_request = json!({
                                "source": "twitch_auth",
                                "adapter": "twitch",
                                "access_token": access_token_clone,
                                "refresh_token": refresh_token_clone,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            // Publish a special event that the service can react to
                            let event = crate::StreamEvent::new(
                                "system",
                                "save_tokens_request",
                                save_request,
                            );

                            if let Err(e) = event_bus.publish(event).await {
                                error!("Failed to request token save: {}", e);
                            } else {
                                info!("Sent request to save tokens to persistent storage");
                            }
                        });

                        // Save tokens to secure store
                        // NOTE: Adapter name was previously unused - removed

                        // Use the auth manager's method to get tokens for storage
                        // TODO: This creates tokens but doesn't actually store them - incomplete feature
                        let _tokens = if let Some((access, refresh)) = self_clone
                            .auth_manager
                            .read()
                            .await
                            .get_token_for_storage()
                            .await
                        {
                            json!({
                                "access_token": access,
                                "refresh_token": refresh,
                            })
                        } else {
                            // Fallback to the extracted tokens if the auth manager method fails
                            json!({
                                "access_token": access_token.clone(),
                                "refresh_token": refresh_token.clone(),
                            })
                        };

                        // Update our local config with the new tokens
                        let mut config = self_clone.config.write().await;
                        config.access_token = Some(access_token.clone());
                        config.refresh_token = refresh_token.clone();

                        // Create a config object with tokens to save in the settings
                        // This is important because we need to explicitly include access_token and refresh_token
                        // in the config that gets stored
                        let config_with_tokens = json!({
                            "channel_id": config.channel_id,
                            "channel_login": config.channel_login,
                            "access_token": access_token.clone(),
                            "refresh_token": refresh_token.clone(),
                            "poll_interval_ms": config.poll_interval_ms,
                            "monitor_channel_info": config.monitor_channel_info,
                            "monitor_stream_status": config.monitor_stream_status,
                        });

                        // Get display name and description for complete settings
                        let display_name = "Twitch".to_string();
                        let description =
                            "Connects to Twitch to receive stream information and events"
                                .to_string();
                        drop(config); // Release the lock

                        // Create a complete settings object
                        // This is the same structure expected by update_adapter_settings
                        // TODO: This settings object is created but never used - incomplete feature or leftover
                        let _adapter_settings = json!({
                            "enabled": true,
                            "config": config_with_tokens,
                            "display_name": display_name,
                            "description": description
                        });

                        // Instead of trying to directly update the system with an event mechanism that's complex,
                        // let's create a scheduled save task that runs in the background with a simple approach

                        // Make this update visible in logs so we know tokens have been set
                        info!("Twitch authentication successful, tokens are set in adapter config");
                        info!("To persist these tokens, please update adapter settings via UI");

                        // Publish event for UI to provide guidance
                        let ui_guidance_payload = json!({
                            "event": "auth_save_instruction",
                            "message": "Authentication successful! To persist these tokens, please visit Settings and click Save on the Twitch adapter.",
                            "requires_save": true,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        if let Err(e) = self_clone
                            .base
                            .publish_event("auth.guidance", ui_guidance_payload)
                            .await
                        {
                            warn!("Failed to publish auth guidance: {}", e);
                        }

                        // Send a more user-friendly notification about token saving
                        let token_saved_payload = json!({
                            "event": "tokens_saved",
                            "message": "Authentication successful! Twitch access token has been saved.",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        if let Err(e) = self_clone
                            .base
                            .publish_event("auth.tokens_saved", token_saved_payload)
                            .await
                        {
                            warn!("Failed to publish token saved event: {}", e);
                        }

                        // Publish success event
                        let event_payload = json!({
                            "event": "authentication_success",
                            "message": "Successfully authenticated with Twitch",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        if let Err(e) = self_clone
                            .base
                            .publish_event("auth.success", event_payload)
                            .await
                        {
                            error!("Failed to publish success event: {}", e);
                        }

                        // Check if the auth state shows authenticated
                        info!("Verifying authentication state...");
                        let auth_manager = self_clone.auth_manager.read().await;
                        if auth_manager.is_authenticated().await {
                            info!("Authentication state verified: is_authenticated=true");
                            
                            // No need to explicitly start EventSub here anymore:
                            // The auth callback will trigger EventSub activation reactively
                            // when it receives the AuthenticationSuccess event
                        } else {
                            warn!("Authentication state inconsistency: is_authenticated=false despite successful token");
                        }

                        return;
                    }
                    Err(e) => {
                        // Check if it's just a pending error
                        if e.to_string().contains("authorization_pending") {
                            // Still waiting, continue polling
                            debug!("Waiting for user to authorize (authorization_pending)");
                            tokio::time::sleep(Duration::from_millis(DEVICE_CODE_POLLING_MS)).await;
                            continue;
                        } else if e.to_string().contains("expired") {
                            error!("Device code has expired: {}", e);

                            // Reset auth state
                            if let Err(reset_err) =
                                self_clone.auth_manager.write().await.reset_auth_state()
                            {
                                error!("Failed to reset auth state: {}", reset_err);
                            }

                            // Publish expired event
                            let event_payload = json!({
                                "event": "authentication_failed",
                                "error": format!("Device code expired: {}", e),
                                "message": "Twitch authentication code expired",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            if let Err(event_err) = self_clone
                                .base
                                .publish_event("auth.failed", event_payload)
                                .await
                            {
                                error!("Failed to publish expired event: {}", event_err);
                            }

                            return;
                        } else {
                            // Real error
                            error!("Device auth polling failed: {}", e);

                            // Log more debug info to console, but don't flood
                            tokio::time::sleep(Duration::from_millis(DEVICE_CODE_POLLING_MS)).await;
                            continue;
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

        // Flag to track if we've already started auth
        let auth_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let auth_started_time = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Loop until shutdown or disconnected
        loop {
            // Check for shutdown signal with a short timeout
            let maybe_shutdown =
                tokio::time::timeout(Duration::from_millis(10), shutdown_rx.recv()).await;

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
            let is_authenticated = auth_manager.is_authenticated().await;
            drop(auth_manager); // Release the lock

            if !is_authenticated {
                // Before checking if we've started, check if we have access token in config
                // This handles the case where authentication completed in a different task
                let config = self.config.read().await.clone();
                if config.access_token.is_some() {
                    // We have a token in config, try to restore it
                    info!("Found access token in config, restoring authentication");

                    let access_token = config.access_token.unwrap();
                    let refresh_token = config.refresh_token.clone();

                    // Try to restore auth from the saved tokens
                    match self
                        .auth_manager
                        .write()
                        .await
                        .restore_from_saved_tokens(access_token, refresh_token)
                        .await
                    {
                        Ok(_) => {
                            info!("Successfully restored authentication from saved tokens");
                            // Continue the flow - this will now find us authenticated
                            continue;
                        }
                        Err(e) => {
                            warn!("Failed to restore authentication from saved tokens: {}", e);
                            // Fall through to regular auth flow
                        }
                    }
                }

                // Check if we've already started auth process
                let auth_started_val = auth_started.load(std::sync::atomic::Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if auth_started_val {
                    // Check if enough time has passed since we started auth
                    let auth_start_time =
                        auth_started_time.load(std::sync::atomic::Ordering::Relaxed);
                    let elapsed = now.saturating_sub(auth_start_time);

                    // If it's been more than 30 minutes, reset and try again
                    if elapsed > 1800 {
                        info!("Twitch auth has been pending for more than 30 minutes, restarting");
                        auth_started.store(false, std::sync::atomic::Ordering::Relaxed);
                    } else {
                        // Check if we might have a token now in config
                        let config = self.config.read().await.clone();
                        if config.access_token.is_some() {
                            info!("Authentication appears to have completed in a separate task, continuing");
                            continue;
                        }

                        // Just wait and continue polling
                        debug!("Waiting for user to complete Twitch authentication (started {} seconds ago)", elapsed);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }

                if !auth_started_val {
                    // Need to authenticate for the first time
                    info!("Twitch adapter is not authenticated, starting authentication...");

                    match self.authenticate().await {
                        Ok(_) => {
                            info!("Successfully started Twitch authentication process");
                            auth_started.store(true, std::sync::atomic::Ordering::Relaxed);
                            auth_started_time.store(now, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(e) => {
                            error!("Failed to start Twitch authentication: {}", e);
                            tokio::time::sleep(Duration::from_secs(30)).await;
                        }
                    }
                }

                // Short wait before continuing
                tokio::time::sleep(Duration::from_millis(1000)).await;
                continue;
            }

            // Get the auth manager first, then check if token needs refresh
            // IMPORTANT: Need to use a write lock when refreshing tokens
            let auth_manager = self.auth_manager.write().await;
            
            // Check if token needs refresh FIRST, before using it
            info!("Checking if token needs refresh before using it");
            let refresh_result = auth_manager.refresh_token_if_needed().await;
            
            // Now get the (potentially refreshed) token
            let token = match auth_manager.get_token().await {
                Some(token) => token,
                None => {
                    // This shouldn't happen as we just checked is_authenticated
                    error!("No token available despite being authenticated");
                    drop(auth_manager); // Release the write lock
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Store token for comparison after refresh attempt
            let token_clone = Some(token.clone());

            // If we got a refreshed token, update it in TokenManager
            if refresh_result.is_ok() {
                // Check if token actually changed
                let new_token = auth_manager.get_token().await;

                // If token was refreshed, update it in TokenManager
                if let Some(new_token) = new_token {
                    if let Some(old_token) = token_clone {
                        // Check if token changed
                        let token_changed = new_token.access_token.secret()
                            != old_token.access_token.secret()
                            || new_token.refresh_token.as_ref().map(|t| t.secret())
                                != old_token.refresh_token.as_ref().map(|t| t.secret());

                        if token_changed {
                            debug!("Token was refreshed, updating TokenManager");

                            // Update in TokenManager
                            if let Some(tm) = &self.token_manager {
                                // Extract tokens
                                let access_token = new_token.access_token.secret().to_string();
                                let refresh_token = new_token
                                    .refresh_token
                                    .as_ref()
                                    .map(|t| t.secret().to_string());

                                // Create TokenData
                                let mut token_data =
                                    TokenData::new(access_token.clone(), refresh_token.clone());

                                // Set expiration if available
                                let expires_in = new_token.expires_in().as_secs();
                                
                                // Default to 4 hours (14400 seconds) if no expiration is provided
                                let expires_to_use = if expires_in > 0 {
                                    info!("Refreshed token has expiration time: {} seconds ({} minutes)", 
                                         expires_in, expires_in / 60);
                                    expires_in
                                } else {
                                    info!("Refreshed token has no expiration time, defaulting to 4 hours");
                                    14400 // Default to 4 hours, which is the standard Twitch token lifetime
                                };
                                
                                // Always set an expiration, even if we have to use the default
                                token_data.set_expiration(expires_to_use);

                                // Track when the refresh token was created for 30-day monitoring
                                token_data.track_refresh_token_created();

                                // Store in token manager
                                match tm.store_tokens("twitch", token_data).await {
                                    Ok(_) => debug!(
                                        "Successfully updated tokens in TokenManager after refresh"
                                    ),
                                    Err(e) => {
                                        warn!("Failed to update tokens in TokenManager: {}", e)
                                    }
                                }

                                // Also update config
                                let mut config = self.config.write().await;
                                config.access_token = Some(access_token);
                                config.refresh_token = refresh_token;
                                
                                // Log the token expiry to help with debugging
                                // Can't use await in a closure, so we need to do this differently
                                info!("Refreshed and updated token information in TokenManager");
                                
                                // Use a spawned task to log the token expiration info
                                let tm_clone = tm.clone();
                                tauri::async_runtime::spawn(async move {
                                    match tm_clone.get_tokens("twitch").await {
                                        Ok(Some(token_data)) => {
                                            if let Some(expires_at) = token_data.expires_in {
                                                let now = chrono::Utc::now();
                                                let duration = expires_at.signed_duration_since(now);
                                                let seconds = duration.num_seconds();
                                                info!("Token will expire in {} seconds ({} minutes)", 
                                                    seconds, seconds / 60);
                                            } else {
                                                warn!("Token in TokenManager has no expiration time");
                                            }
                                        },
                                        _ => warn!("Could not retrieve token from TokenManager for expiration check"),
                                    }
                                });
                            }
                        }
                    }
                }
            } else if let Err(e) = refresh_result {
                // Handle refresh errors
                if e.to_string().contains("Not authenticated") {
                    info!(
                        "Not authenticated or token expired, will restart authentication process"
                    );
                } else if e.to_string().contains("30-day expiry limit") {
                    info!("Refresh token has reached its 30-day expiry limit, need to re-authenticate");

                    // Clear any stored tokens since they're now invalid
                    if let Some(tm) = &self.token_manager {
                        if let Err(e) = tm.remove_tokens("twitch").await {
                            warn!("Failed to remove expired tokens from TokenManager: {}", e);
                        } else {
                            info!("Removed expired tokens from TokenManager");
                        }
                    }

                    // Update the config to remove the tokens
                    let mut config = self.config.write().await;
                    config.access_token = None;
                    config.refresh_token = None;

                    // System will automatically trigger authentication flow in next iteration
                    info!("Will automatically begin reauthorization in the next polling cycle");
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
                        }
                        Ok(None) => {
                            debug!("No channel information found");
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to fetch channel info");
                        }
                    }
                } else if let Some(_login) = &config.channel_login {
                    // TODO: Implement lookup by login - variable is unused until implementation
                    warn!("Lookup by channel login not implemented yet");
                }
            }

            // Fetch stream info if enabled
            if config.monitor_stream_status {
                match self
                    .api_client
                    .fetch_stream_info(
                        &token,
                        config.channel_id.as_deref(),
                        config.channel_login.as_deref(),
                    )
                    .await
                {
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
                    }
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
                    }
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

        // Try to load tokens from TokenManager before starting
        let tokens_loaded = self.load_tokens_from_manager().await.unwrap_or(false);
        if tokens_loaded {
            info!("Successfully loaded tokens from TokenManager");
        } else {
            debug!("No tokens loaded from TokenManager, will authenticate if needed");
            
            // Check if we have tokens in config that need to be migrated to TokenManager
            let config = self.config.read().await;
            if let (Some(access_token), refresh_token) = (&config.access_token, &config.refresh_token) {
                info!("Found tokens in config but not in TokenManager, migrating to TokenManager");
                
                if let Some(tm) = &self.token_manager {
                    // Create TokenData
                    let mut token_data = TokenData::new(access_token.clone(), refresh_token.clone());
                    
                    // We don't know expiration time yet, will be updated after validation
                    // But we can track refresh token created time
                    token_data.track_refresh_token_created();
                    
                    // Store in TokenManager
                    match tm.store_tokens("twitch", token_data).await {
                        Ok(_) => info!("Successfully migrated tokens to TokenManager"),
                        Err(e) => error!("Failed to migrate tokens to TokenManager: {}", e),
                    }
                }
                
                info!("Found access token in config, attempting to restore auth state");
                drop(config);

                match self.restore_token_auth_state().await {
                    Ok(_) => {
                        info!("Successfully restored auth state from config");
                        
                        // Update TokenManager with expiry information if we now have a validated token
                        if let Some(tm) = &self.token_manager {
                            let auth_manager = self.auth_manager.read().await;
                            if let Some(token) = auth_manager.get_token().await {
                                // Get token expiry
                                let expires_in = token.expires_in().as_secs();
                                
                                // Update TokenManager with expiry information
                                if let Err(e) = tm.update_tokens("twitch", |mut t| {
                                    // Set expiration time based on the validated token
                                    t.set_expiration(expires_in);
                                    t
                                }).await {
                                    warn!("Failed to update token expiry in TokenManager: {}", e);
                                } else {
                                    info!("Updated TokenManager with token expiry information ({}s)", expires_in);
                                }
                            }
                        }
                    },
                    Err(e) => warn!("Failed to restore auth state from config: {}", e),
                }
            } else {
                drop(config);
            }
        }

        // Create the shutdown channel
        let (_, shutdown_rx) = self.base.create_shutdown_channel().await;

        // Set connected state
        self.base.set_connected(true);

        // Check if we're using EventSub
        let config = self.config.read().await;
        let use_eventsub = config.use_eventsub;
        drop(config);

        if use_eventsub {
            info!("Using EventSub for Twitch events");

            // Start EventSub in a background task
            let self_clone = self.clone();
            let handle = tauri::async_runtime::spawn(async move {
                // Try to start EventSub
                match self_clone.start_eventsub().await {
                    Ok(_) => {
                        info!("Successfully started EventSub");
                    }
                    Err(e) => {
                        error!("Failed to start EventSub: {}", e);

                        // Fall back to polling if EventSub fails
                        let mut config = self_clone.config.write().await;
                        config.use_eventsub = false;
                        drop(config);

                        // Start polling as fallback
                        info!("Falling back to polling for Twitch updates");
                        if let Err(e) = self_clone.poll_twitch_updates(shutdown_rx).await {
                            error!(error = %e, "Error in Twitch update polling");
                        }
                    }
                }
            });

            // Store the event handler task
            self.base.set_event_handler(handle).await;

            info!("Twitch adapter connected with EventSub");
        } else {
            info!("Using polling for Twitch events");

            // Start polling in a background task
            let self_clone = self.clone();
            let handle = tauri::async_runtime::spawn(async move {
                if let Err(e) = self_clone.poll_twitch_updates(shutdown_rx).await {
                    error!(error = %e, "Error in Twitch update polling");
                }
            });

            // Store the event handler task
            self.base.set_event_handler(handle).await;

            info!("Twitch adapter connected and polling for updates");
        }

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

        // Check if we're using EventSub
        let config = self.config.read().await;
        let use_eventsub = config.use_eventsub;
        drop(config);

        if use_eventsub {
            // Stop EventSub client
            if let Err(e) = self.stop_eventsub().await {
                warn!("Error stopping EventSub client: {}", e);
                // Continue with disconnect regardless
            }
        }

        // Stop the event handler (this will stop the polling task if running)
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

        // Check if refresh token is approaching its 30-day expiry (if available)
        // If so, quietly trigger a token refresh to extend the expiry period
        if let Some(tm) = &self.token_manager {
            match tm.refresh_tokens_expire_soon("twitch", 7).await {
                true => {
                    info!("Twitch refresh token is approaching its 30-day expiry limit. Triggering background refresh.");

                    // Get current token and force a refresh
                    let auth_manager = self.auth_manager.read().await;
                    if auth_manager.is_authenticated().await {
                        // Clone what we need before dropping the read lock
                        let auth_manager_clone = Arc::clone(&self.auth_manager);
                        drop(auth_manager);

                        // Spawn a background task to refresh the token
                        tauri::async_runtime::spawn(async move {
                            match auth_manager_clone
                                .write()
                                .await
                                .refresh_token_if_needed()
                                .await
                            {
                                Ok(_) => info!(
                                    "Successfully refreshed token to extend 30-day expiry period"
                                ),
                                Err(e) => warn!(
                                    "Failed to refresh token despite approaching expiry: {}",
                                    e
                                ),
                            }
                        });
                    } else {
                        debug!(
                            "Not authenticated, cannot refresh token despite approaching expiry"
                        );
                    }
                }
                false => {
                    debug!("Refresh token is not approaching expiry or no timestamp available")
                }
            }
        }

        // Check if channel changed
        let channel_changed = current_config.channel_id != new_config.channel_id
            || current_config.channel_login != new_config.channel_login;

        // Check if EventSub setting changed
        let eventsub_changed = current_config.use_eventsub != new_config.use_eventsub;

        // Update our configuration
        let mut current_config = self.config.write().await;
        *current_config = new_config.clone();

        // Handle EventSub setting changes
        if eventsub_changed {
            info!("EventSub setting changed to: {}", new_config.use_eventsub);

            // If we're connected, we need to restart with the new setting
            if self.is_connected() {
                info!("Adapter is connected, will restart with new EventSub setting");

                // Stop existing EventSub if it's running and we're switching to polling
                if !new_config.use_eventsub {
                    if let Err(e) = self.stop_eventsub().await {
                        warn!("Error stopping EventSub: {}", e);
                    }
                }
            }
        }

        // If auth changed, update auth manager
        if auth_changed {
            info!("Authentication info changed, updating auth manager");

            // Get the client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => {
                    info!("Using Twitch Client ID from environment variable");
                    id
                }
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

                let result = auth_manager
                    .restore_from_saved_tokens(
                        access_token.clone(),
                        new_config.refresh_token.clone(),
                    )
                    .await;

                match result {
                    Ok(_) => {
                        info!("Successfully restored and validated authentication tokens");
                    }
                    Err(e) => {
                        // Token validation failed, we need to notify the user
                        warn!(
                            "Failed to restore from saved tokens (invalid or expired): {}",
                            e
                        );

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

                        if let Err(event_err) =
                            self.base.publish_event("auth.token_invalid", payload).await
                        {
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

        // CRITICAL: Maintaining callback integrity across async boundaries
        //
        // This was the source of a significant bug where EventSub wasn't activating
        // on initial authentication. The issue was that we were creating a fresh auth
        // manager for each clone, which meant:
        //
        // 1. Callbacks registered on the original instance were lost in the clones
        // 2. Auth events weren't propagating to the reactive handlers
        // 3. The EventSub activation which depended on auth callbacks wasn't triggered
        //
        // The fix is to share the SAME auth_manager instance across all clones
        // using Arc::clone() instead of creating a new instance. This ensures:
        //
        // 1. All auth callbacks remain registered no matter which clone processes an event
        // 2. The reactive architecture works properly, with auth events triggering appropriate actions
        // 3. EventSub activation happens automatically in response to auth events
        //
        // This pattern should be used for ALL adapter types with callbacks or shared state!
        
        Self {
            base: self.base.clone(),
            config: Arc::clone(&self.config),
            auth_manager: Arc::clone(&self.auth_manager), // Keep the SAME auth manager
            api_client: self.api_client.clone(),
            state: Arc::clone(&self.state),
            token_manager: self.token_manager.clone(),
            eventsub_client: Arc::clone(&self.eventsub_client),
            recovery_manager: self.recovery_manager.clone(),
        }
    }
}

// Implement AdapterRecovery trait
impl AdapterRecovery for TwitchAdapter {
    fn recovery_manager(&self) -> Arc<RecoveryManager> {
        self.recovery_manager
            .clone()
            .expect("Recovery manager not set")
    }

    fn adapter_name(&self) -> &str {
        self.base.name()
    }
}
