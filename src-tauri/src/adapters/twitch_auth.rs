use anyhow::{anyhow, Result};
use serde_json::json;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use twitch_oauth2::{
    id::DeviceCodeResponse, AccessToken, ClientId, DeviceUserTokenBuilder, RefreshToken, Scope,
    TwitchToken, UserToken,
};

// Internal imports
use crate::adapters::{
    common::TraceHelper,
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
    auth_state: Arc<RwLock<AuthState>>,
    /// Event callbacks for auth state changes using the improved callback system
    auth_callbacks: Arc<TwitchAuthCallbackRegistry>,
    /// HTTP client for API requests
    http_client: Arc<dyn HttpClient + Send + Sync>,
}

impl Clone for TwitchAuthManager {
    fn clone(&self) -> Self {
        Self {
            auth_state: Arc::clone(&self.auth_state), // Properly share auth state
            auth_callbacks: Arc::clone(&self.auth_callbacks), // Callbacks properly shared
            http_client: Arc::clone(&self.http_client), // HTTP client properly shared
        }
    }
}

impl TwitchAuthManager {
    /// Create a new auth manager
    pub fn new(_client_id_unused: String) -> Self {
        // _client_id_unused parameter is kept for backward compatibility but not used
        // Client ID is now retrieved from environment variables when needed

        Self {
            auth_state: Arc::new(RwLock::new(AuthState::NotAuthenticated)),
            auth_callbacks: Arc::new(TwitchAuthCallbackRegistry::new()),
            http_client: Arc::new(ReqwestHttpClient::new()),
        }
    }

    /// Create a new auth manager with a custom HTTP client
    pub fn with_http_client(http_client: Arc<dyn HttpClient + Send + Sync>) -> Self {
        Self {
            auth_state: Arc::new(RwLock::new(AuthState::NotAuthenticated)),
            auth_callbacks: Arc::new(TwitchAuthCallbackRegistry::new()),
            http_client,
        }
    }

    /// Register an event callback for auth state changes
    pub async fn register_auth_callback<F>(
        &self,
        callback: F,
    ) -> Result<crate::callback_system::CallbackId>
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
    pub async fn set_device_code(&mut self, device_code: DeviceCodeResponse) -> Result<()> {
        *self.auth_state.write().await = AuthState::PendingDeviceAuth(device_code);
        Ok(())
    }

    /// Set a token in the auth state
    pub async fn set_token(&mut self, token: UserToken) -> Result<()> {
        *self.auth_state.write().await = AuthState::Authenticated(token);
        Ok(())
    }

    /// Reset the auth state to NotAuthenticated
    pub async fn reset_auth_state(&mut self) -> Result<()> {
        *self.auth_state.write().await = AuthState::NotAuthenticated;
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
        )
        .await;

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

            // Start tracing the operation
            let operation_name = "twitch_token_refresh";
            TraceHelper::record_adapter_operation(
                "twitch",
                &format!("{}_start", operation_name),
                Some(serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            // Use our retry helper
            use crate::common::retry::{exponential_backoff, with_jitter, with_retry_and_backoff};

            // For the retry operation, we need to clone these variables
            let token_clone = token.clone();
            let self_clone = self.clone();

            // Execute the token refresh with retry logic
            let result: Result<UserToken, anyhow::Error> = with_retry_and_backoff(
                || {
                    // Create clones for the async block
                    let token = token_clone.clone();
                    let manager = self_clone.clone();

                    Box::pin(async move {
                        // Get client ID from environment - using match instead of map_err
                        let client_id = match get_client_id() {
                            Ok(id) => id,
                            Err(e) => {
                                return Err(anyhow!("Failed to get client ID for token refresh: {}", e));
                            }
                        };

                        // Create HTTP client - using match instead of map_err
                        let http_client = match reqwest::Client::builder()
                            .redirect(reqwest::redirect::Policy::none())
                            .build() 
                        {
                            Ok(client) => client,
                            Err(e) => {
                                return Err(anyhow!("Failed to create HTTP client: {}", e));
                            }
                        };

                        // Record attempt in trace
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_refresh_attempt",
                            Some(serde_json::json!({
                                "has_refresh_token": token.refresh_token.is_some(),
                                "token_expires_in_seconds": token.expires_in().as_secs(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        // Check if we have a refresh token
                        if let Some(refresh_token) = token.refresh_token.clone() {
                            // Use the method that automatically refreshes if needed - using match instead of map_err
                            let refreshed_token = match UserToken::from_existing_or_refresh_token(
                                &http_client,
                                token.access_token.clone(),
                                refresh_token,
                                client_id,
                                None, // client_secret
                            )
                            .await {
                                Ok(token) => token,
                                Err(e) => {
                                    return Err(anyhow!("Failed to validate or refresh token: {}", e));
                                }
                            };

                            info!("Token successfully validated or refreshed");

                            // Log token details without exposing the actual token
                            info!(
                                "Token details: expires_in={}s, has_refresh_token={}",
                                refreshed_token.expires_in().as_secs(),
                                refreshed_token.refresh_token.is_some()
                            );

                            // Update auth state with the refreshed token
                            *manager.auth_state.write().await =
                                AuthState::Authenticated(refreshed_token.clone());

                            // Send refresh event
                            if let Err(e) = manager.send_event(AuthEvent::TokenRefreshed).await {
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
                                payload: token_refresh_payload.clone(),
                            };

                            if let Err(e) = manager.send_event(refresh_details_event).await {
                                warn!("Could not send token refresh details: {}", e);
                            }

                            // Log success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_refresh_success",
                                Some(token_refresh_payload),
                            )
                            .await;

                            Ok(refreshed_token)
                        } else {
                            // No refresh token available - just validate the token
                            info!("No refresh token available, validating access token directly");

                            // Direct token validation - using match instead of map_err
                            match token.access_token.validate_token(&http_client).await {
                                Ok(_) => {},
                                Err(e) => {
                                    return Err(anyhow!("Token validation failed: {}", e));
                                }
                            };

                            info!("Access token is still valid (validated directly)");

                            // Log success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                "token_validation_success",
                                Some(serde_json::json!({
                                    "expires_in_seconds": token.expires_in().as_secs(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            Ok(token.clone())
                        }
                    })
                },
                2, // max attempts
                operation_name,
                with_jitter(exponential_backoff(100, Some(2000))), // Base 100ms, max 2s, with jitter
            )
            .await;

            // Handle the result of the token refresh operation
            match result {
                Ok(_) => {
                    // Record success
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_success", operation_name),
                        Some(serde_json::json!({
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // The token was successfully refreshed or validated
                    Ok(())
                }
                Err(e) => {
                    // Record final failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failed", operation_name),
                        Some(serde_json::json!({
                            "error": e.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Reset auth state
                    *self.auth_state.write().await = AuthState::NotAuthenticated;

                    // Send token expired event
                    if let Err(event_err) = self
                        .send_event(AuthEvent::TokenExpired {
                            error: e.to_string(),
                        })
                        .await
                    {
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
            )
            .await;

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
        )
        .await;

        info!(
            "Attempting to restore from tokens: access_token_len={}, has_refresh_token={}",
            access_token.len(),
            refresh_token.is_some()
        );

        // Get client ID first since this is needed for multiple paths
        let client_id = match get_client_id() {
            Ok(id) => id,
            Err(e) => {
                return Err(anyhow!(
                    "Failed to get client ID for token restoration: {}",
                    e
                ))
            }
        };

        // Create clones for the retry closure
        let self_clone = self.clone();
        let operation_name = "token_restoration";
        let access_token_clone = access_token.clone();
        let refresh_token_clone = refresh_token.clone();

        // Use our retry helper
        use crate::common::retry::with_retry;

        let result: Result<UserToken, anyhow::Error> = with_retry(
            || {
                // Create clones for the async block
                let access_token = access_token_clone.clone();
                let refresh_token = refresh_token_clone.clone();
                let manager = self_clone.clone();
                let client_id = client_id.clone();

                Box::pin(async move {
                    // Create HTTP client for this attempt - using match instead of map_err
                    let http_client = match reqwest::Client::builder()
                        .redirect(reqwest::redirect::Policy::none())
                        .build() {
                        Ok(client) => client,
                        Err(e) => {
                            return Err(anyhow!("Failed to create HTTP client: {}", e));
                        }
                    };

                    // Create access token object
                    let access_token_obj = AccessToken::new(access_token);

                    // The logic branches based on whether we have a refresh token
                    if let Some(refresh_token_str) = refresh_token {
                        // If we have both access token and refresh token
                        let refresh_token_obj = RefreshToken::new(refresh_token_str);

                        // This will validate the token and refresh it if it's expired - using match instead of map_err
                        let token = match UserToken::from_existing_or_refresh_token(
                            &http_client,
                            access_token_obj,
                            refresh_token_obj,
                            client_id,
                            None, // client_secret
                        )
                        .await {
                            Ok(token) => token,
                            Err(e) => {
                                return Err(anyhow!("Failed to restore or refresh token: {}", e));
                            }
                        };

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
                        *manager.auth_state.write().await = AuthState::Authenticated(token.clone());

                        // Create a trace record for successful token restoration
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_restoration_success",
                            Some(serde_json::json!({
                                "expires_in_seconds": token.expires_in().as_secs(),
                                "has_refresh_token": token.refresh_token.is_some(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        // Send a refresh event if it was refreshed
                        if let Err(e) = manager.send_event(AuthEvent::TokenRefreshed).await {
                            warn!("Failed to send token refreshed event: {}", e);
                        }

                        Ok(token)
                    } else {
                        // If we only have access token, try to validate it
                        info!("No refresh token available, validating access token directly");

                        // Validate token directly - using match instead of map_err
                        match access_token_obj.validate_token(&http_client).await {
                            Ok(_) => {},
                            Err(e) => {
                                return Err(anyhow!("Token validation failed: {}", e));
                            }
                        };

                        info!("Successfully validated access token");

                        // Create a UserToken from the validated data - using match instead of map_err
                        let token = match UserToken::from_existing(
                            &http_client,
                            access_token_obj,
                            None, // No refresh token
                            None, // No client secret
                        )
                        .await {
                            Ok(token) => token,
                            Err(e) => {
                                return Err(anyhow!(
                                    "Failed to create UserToken from validated access token: {}",
                                    e
                                ));
                            }
                        };

                        // Store valid token in auth state
                        *manager.auth_state.write().await = AuthState::Authenticated(token.clone());

                        // Log success in trace
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            "token_validation_success",
                            Some(serde_json::json!({
                                "expires_in_seconds": token.expires_in().as_secs(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        Ok(token)
                    }
                })
            },
            2, // max attempts
            operation_name,
        )
        .await;

        // Handle the result
        match result {
            Ok(token) => Ok(token),
            Err(e) => {
                // Log failure in trace
                TraceHelper::record_adapter_operation(
                    "twitch",
                    "token_restoration_failed",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                // Send token expired event before returning error
                if let Err(event_err) = self
                    .send_event(AuthEvent::TokenExpired {
                        error: e.to_string(),
                    })
                    .await
                {
                    warn!("Failed to send token expired event: {}", event_err);
                }

                Err(e)
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
    pub use crate::adapters::tests::twitch_auth_test::*;
}
