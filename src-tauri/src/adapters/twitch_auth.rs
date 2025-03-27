use anyhow::{anyhow, Result};
use serde_json::json;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use twitch_oauth2::{
    id::DeviceCodeResponse, AccessToken, ClientId, DeviceUserTokenBuilder, RefreshToken, Scope,
    TwitchToken, UserToken,
};

// Internal imports
use crate::adapters::{
    common::{AdapterError, BackoffStrategy, RetryOptions, TraceHelper},
    http_client::{HttpClient, ReqwestHttpClient},
};

// Helper struct for token validation response
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
struct ValidatedToken {
    client_id: String,
    login: String,
    scopes: Vec<String>,
    user_id: String,
    expires_in: u64,
}

/// Environment variable name for Twitch Client ID
const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

/// Get the Twitch Client ID from environment
fn get_client_id() -> Result<ClientId> {
    match env::var(TWITCH_CLIENT_ID_ENV) {
        Ok(client_id) if !client_id.is_empty() => Ok(ClientId::new(client_id)),
        Ok(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is empty")),
        Err(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is not set")),
    }
}

/// Get all available Twitch scopes
/// These will be pared down later when we know exactly which ones we need
fn get_scopes() -> Vec<Scope> {
    vec![
        Scope::AnalyticsReadExtensions,
        Scope::AnalyticsReadGames,
        Scope::BitsRead,
        Scope::ChannelBot,
        Scope::ChannelModerate,
        Scope::ChannelReadGoals,
        Scope::ChannelReadHypeTrain,
        Scope::ChannelReadPolls,
        Scope::ChannelReadPredictions,
        Scope::ChannelReadRedemptions,
        Scope::ChannelReadSubscriptions,
        Scope::ChannelReadVips,
        Scope::ChatRead,
        Scope::ModerationRead,
        Scope::ModeratorReadAutomodSettings,
        Scope::ModeratorReadBlockedTerms,
        Scope::ModeratorReadChatSettings,
        Scope::ModeratorReadChatters,
        Scope::ModeratorReadFollowers,
        Scope::ModeratorReadGuestStar,
        Scope::ModeratorReadShieldMode,
        Scope::UserReadBlockedUsers,
        Scope::UserReadBroadcast,
        Scope::UserReadChat,
        Scope::UserReadEmail,
        Scope::UserReadFollows,
        Scope::UserReadSubscriptions,
    ]
}

/// Authentication event for callbacks
#[derive(Clone, Debug)]
pub enum AuthEvent {
    /// Device code received, user needs to go to URL
    DeviceCodeReceived {
        verification_uri: String,
        user_code: String,
        expires_in: u64,
    },
    /// Authentication successful
    AuthenticationSuccess,
    /// Authentication failed
    AuthenticationFailed { error: String },
    /// Token refreshed
    TokenRefreshed,
    /// Token refresh details for tracking expiration
    TokenRefreshDetails {
        expires_in: u64,
        payload: serde_json::Value,
    },
    /// Token expired or refresh failed
    TokenExpired { error: String },
}

impl AuthEvent {
    /// Get a string representation of the event type
    pub fn event_type(&self) -> &'static str {
        match self {
            AuthEvent::DeviceCodeReceived { .. } => "device_code",
            AuthEvent::AuthenticationSuccess => "success",
            AuthEvent::AuthenticationFailed { .. } => "failed",
            AuthEvent::TokenRefreshed => "token_refreshed",
            AuthEvent::TokenRefreshDetails { .. } => "token_refresh_details",
            AuthEvent::TokenExpired { .. } => "token_expired",
        }
    }
}

/// Authentication state
#[derive(Clone, Debug)]
enum AuthState {
    /// Not authenticated
    NotAuthenticated,
    /// In the process of authenticating via device code flow
    PendingDeviceAuth(DeviceCodeResponse),
    /// Successfully authenticated
    Authenticated(UserToken),
}

use crate::adapters::twitch_auth_callback::TwitchAuthCallbackRegistry;

/// Manages Twitch API authentication
pub struct TwitchAuthManager {
    /// Current authentication state
    auth_state: RwLock<AuthState>,
    /// Event callbacks for auth state changes using the improved callback system
    auth_callbacks: Arc<TwitchAuthCallbackRegistry>,
    /// HTTP client for API requests
    http_client: Arc<dyn HttpClient + Send + Sync>,
}

impl Clone for TwitchAuthManager {
    fn clone(&self) -> Self {
        Self {
            auth_state: RwLock::new(AuthState::NotAuthenticated), // State not shared
            auth_callbacks: Arc::clone(&self.auth_callbacks),     // Callbacks properly shared
            http_client: Arc::clone(&self.http_client),           // HTTP client properly shared
        }
    }
}

impl TwitchAuthManager {
    /// Create a new auth manager
    pub fn new(_client_id_unused: String) -> Self {
        // _client_id_unused parameter is kept for backward compatibility but not used
        // Client ID is now retrieved from environment variables when needed

        Self {
            auth_state: RwLock::new(AuthState::NotAuthenticated),
            auth_callbacks: Arc::new(TwitchAuthCallbackRegistry::new()),
            http_client: Arc::new(ReqwestHttpClient::new()),
        }
    }

    /// Create a new auth manager with a custom HTTP client
    pub fn with_http_client(http_client: Arc<dyn HttpClient + Send + Sync>) -> Self {
        Self {
            auth_state: RwLock::new(AuthState::NotAuthenticated),
            auth_callbacks: Arc::new(TwitchAuthCallbackRegistry::new()),
            http_client,
        }
    }

    /// Register an event callback for auth state changes
    pub async fn register_auth_callback<F>(&self, callback: F) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(AuthEvent) -> Result<()> + Send + Sync + 'static,
    {
        let id = self.auth_callbacks.register(callback).await;
        info!(callback_id = %id, "Registered auth callback");
        Ok(id)
    }

    /// Send an event to all registered callbacks
    async fn send_event(&self, event: AuthEvent) -> Result<()> {
        // Use the callback registry to trigger all callbacks
        let count = self.auth_callbacks.trigger(event.clone()).await?;
        
        // Provide a detailed log message for better diagnostics
        info!(
            event_type = %event.event_type(),
            callbacks_executed = count,
            "Auth event dispatched to {} callbacks",
            count
        );
        
        Ok(())
    }
    
    /// Manually trigger an auth event for reactive handling
    pub async fn trigger_auth_event(&self, event: AuthEvent) -> Result<()> {
        info!("Manually triggering auth event: {:?}", event);
        self.send_event(event).await
    }

    /// Get the scopes for Twitch authentication
    pub fn get_scopes(&self) -> Result<Vec<Scope>> {
        Ok(get_scopes())
    }

    /// Set the device code in the auth state
    pub fn set_device_code(&mut self, device_code: DeviceCodeResponse) -> Result<()> {
        *self.auth_state.get_mut() = AuthState::PendingDeviceAuth(device_code);
        Ok(())
    }

    /// Set a token in the auth state
    pub fn set_token(&mut self, token: UserToken) -> Result<()> {
        *self.auth_state.get_mut() = AuthState::Authenticated(token);
        Ok(())
    }

    /// Reset the auth state to NotAuthenticated
    pub fn reset_auth_state(&mut self) -> Result<()> {
        *self.auth_state.get_mut() = AuthState::NotAuthenticated;
        Ok(())
    }

    /// Start device code authentication flow
    pub async fn start_device_auth(&self, _scopes: Vec<Scope>) -> Result<DeviceCodeResponse> {
        info!("Starting Twitch device auth flow");

        // Get client ID from environment
        let client_id = get_client_id()?;

        // Create HTTP client for auth requests - using reqwest directly as recommended by the twitch_oauth2 crate
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        // Get all scopes
        let all_scopes = get_scopes();

        // Create the builder with all scopes
        let mut builder = DeviceUserTokenBuilder::new(client_id, all_scopes);

        // Start the device code flow using the library's built-in functionality
        let device_code = builder.start(&http_client).await?;

        // Log the verification URI and code for debugging
        info!("=== TWITCH AUTHENTICATION REQUIRED ===");
        info!("Visit: {}", device_code.verification_uri);
        info!("Enter code: {}", device_code.user_code);
        info!("Code expires in {} seconds", device_code.expires_in);
        info!("======================================");

        // Store the device code in auth state
        *self.auth_state.write().await = AuthState::PendingDeviceAuth(device_code.clone());

        // Send event for UI to display to user
        match self
            .send_event(AuthEvent::DeviceCodeReceived {
                verification_uri: device_code.verification_uri.clone(),
                user_code: device_code.user_code.clone(),
                expires_in: device_code.expires_in,
            })
            .await
        {
            Ok(_) => info!("Successfully sent device code event"),
            Err(e) => error!("Failed to send device code event: {}", e),
        };

        Ok(device_code.clone())
    }

    /// Poll for device code completion
    pub async fn poll_device_auth(&self) -> Result<UserToken> {
        let auth_state = self.auth_state.read().await.clone();

        match auth_state {
            AuthState::PendingDeviceAuth(_device_code) => {
                // Get client ID from environment
                let client_id = get_client_id()?;

                // Create HTTP client for auth requests - using reqwest directly as recommended by the twitch_oauth2 crate
                let http_client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?;

                // Get all scopes
                let all_scopes = get_scopes();

                // Create a builder with the same scopes
                let mut builder = DeviceUserTokenBuilder::new(client_id, all_scopes);

                // Poll the device code using the library's built-in functionality
                match builder
                    .wait_for_code(&http_client, tokio::time::sleep)
                    .await
                {
                    Ok(token) => {
                        // Update auth state
                        *self.auth_state.write().await = AuthState::Authenticated(token.clone());

                        // Send success event
                        self.send_event(AuthEvent::AuthenticationSuccess).await?;

                        Ok(token)
                    }
                    Err(e) => {
                        // Check if it's just a "still waiting" error
                        if e.to_string().contains("authorization_pending")
                            || e.to_string()
                                .contains("The authorization request is still pending")
                        {
                            return Err(anyhow!("authorization_pending"));
                        }

                        // Real error
                        error!("Authentication failed: {}", e);

                        // Reset auth state
                        *self.auth_state.write().await = AuthState::NotAuthenticated;

                        // Send failure event
                        self.send_event(AuthEvent::AuthenticationFailed {
                            error: e.to_string(),
                        })
                        .await?;

                        Err(anyhow!("Failed to authenticate: {}", e))
                    }
                }
            }
            _ => Err(anyhow!("Not in device authentication state")),
        }
    }


    /// Get the current token if authenticated
    pub async fn get_token(&self) -> Option<UserToken> {
        let auth_state = self.auth_state.read().await.clone();

        match auth_state {
            AuthState::Authenticated(token) => Some(token),
            _ => None,
        }
    }

    /// Check if we have a valid token
    pub async fn is_authenticated(&self) -> bool {
        self.get_token().await.is_some()
    }

    /// Check if we're in the middle of a device code auth flow
    pub async fn is_pending_device_auth(&self) -> bool {
        let auth_state = self.auth_state.read().await.clone();
        matches!(auth_state, AuthState::PendingDeviceAuth(_))
    }

    /// Refresh the token if needed or force a refresh
    /// Uses the new from_existing_or_refresh_token method from twitch_oauth2 crate
    pub async fn refresh_token_if_needed(&self) -> Result<()> {
        // Create trace context for token refresh check
        TraceHelper::record_adapter_operation(
            "twitch",
            "refresh_token_check",
            Some(serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        let auth_state = self.auth_state.read().await.clone();

        if let AuthState::Authenticated(token) = auth_state {
            // Log current token details
            let expires_in_secs = token.expires_in().as_secs();
            let expires_in_mins = expires_in_secs / 60;
            let expires_in_hours = expires_in_mins / 60;
            
            info!(
                "Current token: expires_in={}s ({}m or {}h), has_refresh_token={}",
                expires_in_secs,
                expires_in_mins,
                expires_in_hours,
                token.refresh_token.is_some()
            );

            // Define retry options for token refresh
            let retry_options = RetryOptions::new(
                2, // Only try twice for token refresh
                BackoffStrategy::Exponential {
                    base_delay: std::time::Duration::from_millis(100),
                    max_delay: std::time::Duration::from_secs(2),
                },
                true, // Add jitter
            );
            
            // Implement direct sequential retry logic
            let operation_name = "token_refresh";
            let self_clone = self.clone();
            let token_clone = token.clone();
            
            // Start tracing the operation
            TraceHelper::record_adapter_operation(
                "twitch",
                &format!("{}_start", operation_name),
                Some(serde_json::json!({
                    "max_attempts": retry_options.max_attempts,
                    "backoff": format!("{:?}", retry_options.backoff),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            ).await;
            
            // Initialize result
            let mut result = None;
            let mut last_error = None;
            
            // Retry loop
            for attempt in 1..=retry_options.max_attempts {
                // Record attempt
                TraceHelper::record_adapter_operation(
                    "twitch",
                    &format!("{}_attempt", operation_name),
                    Some(serde_json::json!({
                        "attempt": attempt,
                        "max_attempts": retry_options.max_attempts,
                    })),
                ).await;
                
                debug!(
                    attempt = attempt,
                    "Attempting to refresh token (attempt {}/{})",
                    attempt,
                    retry_options.max_attempts
                );
                
                // Get client ID from environment
                let client_id = match get_client_id() {
                    Ok(id) => id,
                    Err(e) => {
                        let error = AdapterError::from_anyhow_error(
                            "config",
                            "Failed to get client ID for token refresh",
                            e
                        );
                        
                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "max_attempts": retry_options.max_attempts,
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Store last error for return if all attempts fail
                        last_error = Some(error);
                        
                        // If this is the last attempt, we're done with errors
                        if attempt == retry_options.max_attempts {
                            break;
                        }
                        
                        // Calculate delay for the next attempt
                        let delay = retry_options.get_delay(attempt);
                        debug!(
                            attempt = attempt,
                            delay_ms = delay.as_millis(),
                            "Retrying after delay"
                        );
                        
                        // Wait before next attempt
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                };
                
                // Create HTTP client for this attempt
                let http_client = match reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build() {
                    Ok(client) => client,
                    Err(e) => {
                        let error = AdapterError::connection_with_source(
                            "Failed to create HTTP client for token refresh",
                            e
                        );
                        
                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "max_attempts": retry_options.max_attempts,
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Store last error for return if all attempts fail
                        last_error = Some(error);
                        
                        // If this is the last attempt, we're done with errors
                        if attempt == retry_options.max_attempts {
                            break;
                        }
                        
                        // Calculate delay for the next attempt
                        let delay = retry_options.get_delay(attempt);
                        debug!(
                            attempt = attempt,
                            delay_ms = delay.as_millis(),
                            "Retrying after delay"
                        );
                        
                        // Wait before next attempt
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                };
                
                // Check if we have a refresh token
                if let Some(refresh_token) = token_clone.refresh_token.clone() {
                    // Log that we're attempting to refresh the token
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        "token_refresh_attempt",
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "has_refresh_token": true,
                            "token_expires_in_seconds": token_clone.expires_in().as_secs(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                    
                    // Use the method that automatically refreshes if needed
                    match UserToken::from_existing_or_refresh_token(
                        &http_client,
                        token_clone.access_token.clone(),
                        refresh_token,
                        client_id,
                        None, // client_secret
                    ).await {
                        Ok(refreshed_token) => {
                            info!("Token successfully validated or refreshed");
                            
                            // Log token details without exposing the actual token
                            info!(
                                "Token details: expires_in={}s, has_refresh_token={}",
                                refreshed_token.expires_in().as_secs(),
                                refreshed_token.refresh_token.is_some()
                            );
                            
                            // Update auth state with the refreshed token
                            *self_clone.auth_state.write().await = AuthState::Authenticated(refreshed_token.clone());
                            
                            // Send refresh event
                            if let Err(e) = self_clone.send_event(AuthEvent::TokenRefreshed).await {
                                warn!("Failed to send token refreshed event: {}", e);
                            }
                            
                            // Create token refresh details event for TokenManager update
                            let expires_in_secs = refreshed_token.expires_in().as_secs();
                            let token_refresh_payload = json!({
                                "event": "token_refresh_details",
                                "access_token_len": refreshed_token.access_token.secret().len(),
                                "has_refresh_token": refreshed_token.refresh_token.is_some(),
                                "expires_in_seconds": expires_in_secs,
                                "expires_in_minutes": expires_in_secs / 60,
                                "expires_in_hours": expires_in_secs / 3600,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            // Send token refresh details event
                            let refresh_details_event = AuthEvent::TokenRefreshDetails { 
                                expires_in: expires_in_secs,
                                payload: token_refresh_payload.clone()
                            };
                            
                            if let Err(e) = self_clone.send_event(refresh_details_event).await {
                                warn!("Could not send token refresh details: {}", e);
                            }
                            
                            // Log success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_refresh_success",
                                Some(token_refresh_payload),
                            ).await;
                            
                            // Record success and return result
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            result = Some(Ok(refreshed_token));
                            break;
                        },
                        Err(e) => {
                            // Enhanced error logging
                            error!(
                                attempt = attempt,
                                error = %e,
                                "Failed to validate or refresh token"
                            );
                            
                            // Create structured error
                            let error = AdapterError::from_anyhow_error("auth", 
                                format!("Failed to validate or refresh token (attempt {})", attempt),
                                anyhow::anyhow!(e)
                            );
                            
                            // Log the failure in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_refresh_failure",
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            // Store last error for return if all attempts fail
                            last_error = Some(error);
                            
                            // If this is the last attempt, we're done with errors
                            if attempt == retry_options.max_attempts {
                                break;
                            }
                            
                            // Calculate delay for the next attempt
                            let delay = retry_options.get_delay(attempt);
                            debug!(
                                attempt = attempt,
                                delay_ms = delay.as_millis(),
                                "Retrying after delay"
                            );
                            
                            // Wait before next attempt
                            tokio::time::sleep(delay).await;
                        }
                    }
                } else {
                    // No refresh token available - just validate the token
                    info!("No refresh token available, validating access token directly");
                    
                    // Log the validation attempt in trace
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        "token_validation_attempt",
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "has_refresh_token": false,
                            "token_expires_in_seconds": token_clone.expires_in().as_secs(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                    
                    match token_clone.access_token.validate_token(&http_client).await {
                        Ok(_) => {
                            info!("Access token is still valid (validated directly)");
                            
                            // Log success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_validation_success",
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "expires_in_seconds": token_clone.expires_in().as_secs(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            // Record success and return result
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            // Return the original token since it's still valid
                            result = Some(Ok(token_clone.clone()));
                            break;
                        },
                        Err(e) => {
                            error!(
                                attempt = attempt,
                                error = %e,
                                "Token validation failed"
                            );
                            
                            // Create structured error
                            let error = AdapterError::from_anyhow_error("auth", 
                                format!("Token validation failed (attempt {})", attempt),
                                anyhow::anyhow!(e)
                            );
                            
                            // Log the failure in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_validation_failure",
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            // Store last error for return if all attempts fail
                            last_error = Some(error);
                            
                            // If this is the last attempt, we're done with errors
                            if attempt == retry_options.max_attempts {
                                break;
                            }
                            
                            // Calculate delay for the next attempt
                            let delay = retry_options.get_delay(attempt);
                            debug!(
                                attempt = attempt,
                                delay_ms = delay.as_millis(),
                                "Retrying after delay"
                            );
                            
                            // Wait before next attempt
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }
            
            // Process the final result
            let result = match result {
                Some(ok_result) => ok_result,
                None => {
                    // Record final failure
                    let error_msg = match last_error {
                        Some(ref e) => e.to_string(),
                        None => "Unknown error during token refresh".to_string(),
                    };
                    
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_all_attempts_failed", operation_name),
                        Some(serde_json::json!({
                            "max_attempts": retry_options.max_attempts,
                            "error": error_msg,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                    
                    // Return the last error
                    Err(last_error.unwrap_or_else(|| 
                        AdapterError::auth("Failed to refresh token after all retry attempts")
                    ))
                }
            };
            
            // Handle the result of the token refresh operation
            match result {
                Ok(_) => {
                    // The token was successfully refreshed or validated
                    Ok(())
                },
                Err(e) => {
                    // Reset auth state
                    *self.auth_state.write().await = AuthState::NotAuthenticated;
                    
                    // Send token expired event
                    if let Err(event_err) = self.send_event(AuthEvent::TokenExpired {
                        error: e.to_string(),
                    }).await {
                        warn!("Failed to send token expired event: {}", event_err);
                    }
                    
                    // Return the error
                    Err(anyhow!("Failed to refresh or validate token: {}", e))
                }
            }
        } else {
            error!("Cannot refresh token: not authenticated");
            
            // Log the error in trace
            TraceHelper::record_adapter_operation(
                "twitch",
                "token_refresh_error_not_authenticated",
                Some(serde_json::json!({
                    "error": "Not authenticated",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            ).await;
            
            Err(anyhow!("Not authenticated"))
        }
    }

    /// Restore authentication from saved tokens
    /// Uses the new from_existing_or_refresh_token method from twitch_oauth2 crate
    pub async fn restore_from_saved_tokens(
        &self,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<UserToken> {
        // Create trace context for token restoration
        TraceHelper::record_adapter_operation(
            "twitch",
            "restore_auth_tokens",
            Some(serde_json::json!({
                "access_token_len": access_token.len(),
                "has_refresh_token": refresh_token.is_some(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        info!(
            "Attempting to restore from tokens: access_token_len={}, has_refresh_token={}",
            access_token.len(),
            refresh_token.is_some()
        );

        // Define retry options for token restoration
        let retry_options = RetryOptions::new(
            2, // Only try twice for token restoration
            BackoffStrategy::Constant(std::time::Duration::from_secs(1)), 
            true, // Add jitter
        );
        
        // Implement direct sequential retry logic
        let operation_name = "token_restoration";
        let self_clone = self.clone();
        
        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": retry_options.max_attempts,
                "backoff": format!("{:?}", retry_options.backoff),
                "access_token_len": access_token.len(),
                "has_refresh_token": refresh_token.is_some(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        ).await;
        
        // Initialize result
        let mut result = None;
        let mut last_error = None;
        
        // Retry loop
        for attempt in 1..=retry_options.max_attempts {
            // Record attempt
            TraceHelper::record_adapter_operation(
                "twitch",
                &format!("{}_attempt", operation_name),
                Some(serde_json::json!({
                    "attempt": attempt,
                    "max_attempts": retry_options.max_attempts,
                })),
            ).await;
            
            // Log the attempt
            debug!(
                attempt = attempt,
                "Attempting to restore token (attempt {}/{})",
                attempt,
                retry_options.max_attempts
            );
            
            // Create HTTP client for this attempt
            let http_client = match reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build() {
                Ok(client) => client,
                Err(e) => {
                    let error = AdapterError::connection_with_source(
                        "Failed to create HTTP client for token restoration",
                        e
                    );
                    
                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
                    
                    // Store last error for return if all attempts fail
                    last_error = Some(error);
                    
                    // If this is the last attempt, we're done with errors
                    if attempt == retry_options.max_attempts {
                        break;
                    }
                    
                    // Calculate delay for the next attempt
                    let delay = retry_options.get_delay(attempt);
                    debug!(
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        "Retrying after delay"
                    );
                    
                    // Wait before next attempt
                    tokio::time::sleep(delay).await;
                    continue;
                }
            };
            
            // Create access token object
            let access_token_obj = AccessToken::new(access_token.clone());
            
            if let Some(refresh_token_str) = refresh_token.clone() {
                // If we have both access token and refresh token, use the new method
                let refresh_token_obj = RefreshToken::new(refresh_token_str);
                
                // Get client ID
                let client_id = match get_client_id() {
                    Ok(id) => id,
                    Err(e) => {
                        let error = AdapterError::from_anyhow_error(
                            "config",
                            "Failed to get client ID for token restoration",
                            e
                        );
                        
                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "max_attempts": retry_options.max_attempts,
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Store last error for return if all attempts fail
                        last_error = Some(error);
                        
                        // If this is the last attempt, we're done with errors
                        if attempt == retry_options.max_attempts {
                            break;
                        }
                        
                        // Calculate delay for the next attempt
                        let delay = retry_options.get_delay(attempt);
                        debug!(
                            attempt = attempt,
                            delay_ms = delay.as_millis(),
                            "Retrying after delay"
                        );
                        
                        // Wait before next attempt
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                };
                
                // This will validate the token and refresh it if it's expired
                match UserToken::from_existing_or_refresh_token(
                    &http_client,
                    access_token_obj,
                    refresh_token_obj,
                    client_id,
                    None, // client_secret
                ).await {
                    Ok(token) => {
                        // Successfully restored or refreshed token
                        info!("Successfully restored or refreshed token");
                        
                        // Log token details
                        info!(
                            "Token details: expires_in={}s, has_refresh_token={}, user_id={}",
                            token.expires_in().as_secs(),
                            token.refresh_token.is_some(),
                            token.user_id
                        );
                        
                        // Store the token in the auth state
                        *self_clone.auth_state.write().await = AuthState::Authenticated(token.clone());
                        
                        // Send a refresh event if it was refreshed
                        if let Err(e) = self_clone.send_event(AuthEvent::TokenRefreshed).await {
                            warn!("Failed to send token refreshed event: {}", e);
                        }
                        
                        // Create a trace record for successful token restoration
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_restoration_success",
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "expires_in_seconds": token.expires_in().as_secs(),
                                "has_refresh_token": token.refresh_token.is_some(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Record success and set result
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_success", operation_name),
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        result = Some(Ok(token));
                        break;
                    },
                    Err(e) => {
                        // Failed to restore or refresh token
                        error!(
                            attempt = attempt,
                            error = %e,
                            "Failed to restore or refresh token"
                        );
                        
                        // Create structured error
                        let error = AdapterError::from_anyhow_error("auth", 
                            format!("Failed to restore or refresh token (attempt {})", attempt),
                            anyhow::anyhow!(e)
                        );
                        
                        // Log the failure in trace
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_restoration_failure",
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Store last error for return if all attempts fail
                        last_error = Some(error);
                        
                        // If this is the last attempt, we're done with errors
                        if attempt == retry_options.max_attempts {
                            break;
                        }
                        
                        // Calculate delay for the next attempt
                        let delay = retry_options.get_delay(attempt);
                        debug!(
                            attempt = attempt,
                            delay_ms = delay.as_millis(),
                            "Retrying after delay"
                        );
                        
                        // Wait before next attempt
                        tokio::time::sleep(delay).await;
                    }
                }
            } else {
                // If we only have access token, try to validate it
                info!("No refresh token available, validating access token directly");
                
                match access_token_obj.validate_token(&http_client).await {
                    Ok(_) => {
                        info!("Successfully validated access token");
                        
                        // Create a UserToken from the validated data
                        let token = match UserToken::from_existing(
                            &http_client,
                            access_token_obj,
                            None, // No refresh token
                            None, // No client secret
                        ).await {
                            Ok(t) => t,
                            Err(e) => {
                                // Create structured error
                                let error = AdapterError::from_anyhow_error("auth", 
                                    "Failed to create UserToken from validated access token",
                                    anyhow::anyhow!(e)
                                );
                                
                                // Record failure
                                TraceHelper::record_adapter_operation(
                                    "twitch",
                                    &format!("{}_failure", operation_name),
                                    Some(serde_json::json!({
                                        "attempt": attempt,
                                        "max_attempts": retry_options.max_attempts,
                                        "error": error.to_string(),
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                ).await;
                                
                                // Store last error for return if all attempts fail
                                last_error = Some(error);
                                
                                // If this is the last attempt, we're done with errors
                                if attempt == retry_options.max_attempts {
                                    break;
                                }
                                
                                // Calculate delay for the next attempt
                                let delay = retry_options.get_delay(attempt);
                                debug!(
                                    attempt = attempt,
                                    delay_ms = delay.as_millis(),
                                    "Retrying after delay"
                                );
                                
                                // Wait before next attempt
                                tokio::time::sleep(delay).await;
                                continue;
                            }
                        };
                        
                        // Store valid token in auth state
                        *self_clone.auth_state.write().await = AuthState::Authenticated(token.clone());
                        
                        // Log success in trace
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_validation_success",
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "expires_in_seconds": token.expires_in().as_secs(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Record success and set result
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_success", operation_name),
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        result = Some(Ok(token));
                        break;
                    },
                    Err(e) => {
                        // Failed to validate access token
                        error!(
                            attempt = attempt,
                            error = %e,
                            "Token validation failed"
                        );
                        
                        // Create structured error
                        let error = AdapterError::from_anyhow_error("auth", 
                            format!("Token validation failed (attempt {})", attempt),
                            anyhow::anyhow!(e)
                        );
                        
                        // Log failure in trace
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_validation_failure",
                            Some(serde_json::json!({
                                "attempt": attempt,
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
                        
                        // Store last error for return if all attempts fail
                        last_error = Some(error);
                        
                        // If this is the last attempt, we're done with errors
                        if attempt == retry_options.max_attempts {
                            break;
                        }
                        
                        // Calculate delay for the next attempt
                        let delay = retry_options.get_delay(attempt);
                        debug!(
                            attempt = attempt,
                            delay_ms = delay.as_millis(),
                            "Retrying after delay"
                        );
                        
                        // Wait before next attempt
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        // Process the final result
        let result = match result {
            Some(ok_result) => ok_result,
            None => {
                // Record final failure
                let error_msg = match last_error {
                    Some(ref e) => e.to_string(),
                    None => "Unknown error during token restoration".to_string(),
                };
                
                TraceHelper::record_adapter_operation(
                    "twitch",
                    &format!("{}_all_attempts_failed", operation_name),
                    Some(serde_json::json!({
                        "max_attempts": retry_options.max_attempts,
                        "error": error_msg,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                ).await;
                
                // Return the last error
                Err(last_error.unwrap_or_else(|| 
                    AdapterError::auth("Failed to restore token after all retry attempts")
                ))
            }
        };
        
        // Convert from AdapterError back to anyhow::Error for compatibility
        match result {
            Ok(token) => Ok(token),
            Err(e) => {
                // Send token expired event before returning error
                if let Err(event_err) = self
                    .send_event(AuthEvent::TokenExpired {
                        error: e.to_string(),
                    })
                    .await
                {
                    warn!("Failed to send token expired event: {}", event_err);
                }
                
                Err(anyhow!("Failed to restore authentication: {}", e))
            }
        }
    }

    /// Get token details for storage
    pub async fn get_token_for_storage(&self) -> Option<(String, Option<String>)> {
        let auth_state = self.auth_state.read().await.clone();

        match auth_state {
            AuthState::Authenticated(token) => {
                let access_token = token.access_token.secret().to_string();
                let refresh_token = token.refresh_token.as_ref().map(|t| t.secret().to_string());

                Some((access_token, refresh_token))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests for this module have been moved to src/adapters/tests/twitch_auth_test.rs
    pub use crate::adapters::tests::twitch_auth_test::*;
}
