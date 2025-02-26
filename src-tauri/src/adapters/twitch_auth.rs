use anyhow::{anyhow, Result};
use std::env;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use twitch_oauth2::{
    id::DeviceCodeResponse, ClientId, DeviceUserTokenBuilder, Scope, TwitchToken, UserToken,
};

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
        Scope::ChannelReadGoals,
        Scope::ChannelReadHypeTrain,
        Scope::ChannelReadPolls,
        Scope::ChannelReadPredictions,
        Scope::ChannelReadRedemptions,
        Scope::ChannelReadSubscriptions,
        Scope::ChannelReadVips,
        Scope::ModerationRead,
        Scope::ModeratorReadBlockedTerms,
        Scope::ModeratorReadAutomodSettings,
        Scope::ModeratorReadChatSettings,
        Scope::ModeratorReadChatters,
        Scope::ModeratorReadFollowers,
        Scope::ModeratorReadGuestStar,
        Scope::ModeratorReadShieldMode,
        Scope::UserReadBlockedUsers,
        Scope::UserReadBroadcast,
        Scope::UserReadEmail,
        Scope::UserReadFollows,
        Scope::UserReadSubscriptions,
        Scope::ChannelModerate,
        Scope::ChatRead,
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

/// Manages Twitch API authentication
pub struct TwitchAuthManager {
    /// Current authentication state
    auth_state: RwLock<AuthState>,
    /// Event callback for auth state changes
    auth_callback: Option<Box<dyn Fn(AuthEvent) -> Result<()> + Send + Sync>>,
}

// We can't properly clone TwitchAuthManager with callbacks and async locks
// But we can provide a dummy clone that creates a new instance instead
// This is only used in contexts where we don't need the actual state
impl Clone for TwitchAuthManager {
    fn clone(&self) -> Self {
        // We can't properly clone the auth state in a blocking way,
        // so we just create a new instance.
        // Within our application, what matters more is that we clone the token
        // through the config anyway.
        Self {
            auth_state: RwLock::new(AuthState::NotAuthenticated),
            auth_callback: None, // We can't clone the callback
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
            auth_callback: None,
        }
    }

    /// Set an event callback for auth state changes
    pub fn set_auth_callback<F>(&mut self, callback: F)
    where
        F: Fn(AuthEvent) -> Result<()> + Send + Sync + 'static,
    {
        self.auth_callback = Some(Box::new(callback));
    }

    /// Send an event to the callback if set
    async fn send_event(&self, event: AuthEvent) -> Result<()> {
        if let Some(callback) = &self.auth_callback {
            callback(event)?;
        }
        Ok(())
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

        // Create HTTP client for auth requests
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        // Get all scopes
        let all_scopes = get_scopes();

        // Create the builder with all scopes
        let mut builder = DeviceUserTokenBuilder::new(client_id, all_scopes);

        // Start the device code flow
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
                // Create HTTP client
                let http_client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?;

                // Get client ID from environment
                let client_id = get_client_id()?;

                // Get all scopes
                let all_scopes = get_scopes();

                // Create a builder with the same scopes
                let mut builder = DeviceUserTokenBuilder::new(client_id, all_scopes);

                // Poll the device code
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
                        if e.to_string().contains("authorization_pending") {
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

    /// Refresh the token if needed
    pub async fn refresh_token_if_needed(&self) -> Result<()> {
        let auth_state = self.auth_state.read().await.clone();

        if let AuthState::Authenticated(mut token) = auth_state {
            // Check if token is about to expire (within 5 minutes)
            // The compiler indicates expires_in() returns a Duration directly
            let should_refresh = token.expires_in().as_secs() < 300;

            if should_refresh {
                info!("Access token expired or about to expire, refreshing");

                // Create HTTP client
                let http_client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()?;

                // Use the TwitchToken trait's refresh_token method directly
                match token.refresh_token(&http_client).await {
                    Ok(()) => {
                        info!("Successfully refreshed token");

                        // Update auth state with new token (which was refreshed in-place)
                        *self.auth_state.write().await = AuthState::Authenticated(token);

                        // Send refresh event
                        self.send_event(AuthEvent::TokenRefreshed).await?;

                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to refresh token: {}", e);

                        // Reset auth state
                        *self.auth_state.write().await = AuthState::NotAuthenticated;

                        // Send token expired event
                        self.send_event(AuthEvent::TokenExpired {
                            error: e.to_string(),
                        })
                        .await?;

                        Err(anyhow!("Token refresh failed: {}", e))
                    }
                }
            } else {
                // Token is still valid
                Ok(())
            }
        } else {
            // Not authenticated
            Err(anyhow!("Not authenticated"))
        }
    }

    /// Restore authentication from saved tokens
    pub async fn restore_from_saved_tokens(
        &self,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<UserToken> {
        // Create HTTP client for validation
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        // Create access token object
        let access_token_obj = twitch_oauth2::AccessToken::new(access_token);

        // Create an optional refresh token object
        let refresh_token_obj = refresh_token.map(twitch_oauth2::RefreshToken::new);

        // First try the normal token validation approach
        match twitch_oauth2::UserToken::from_token(&http_client, access_token_obj).await {
            Ok(token) => {
                // Token is valid, store it and return
                info!("Successfully validated existing token");
                *self.auth_state.write().await = AuthState::Authenticated(token.clone());
                Ok(token)
            }
            Err(e) => {
                // Instead of complex token reconstruction, just report that token is invalid and needs re-auth
                info!("Token validation failed: {}. Re-authentication required", e);

                // If we have a refresh token, log that we couldn't use it
                if refresh_token_obj.is_some() {
                    warn!("Had a refresh token but couldn't use it automatically. Re-authentication required.");
                }

                // Send token expired event
                if let Err(event_err) = self
                    .send_event(AuthEvent::TokenExpired {
                        error: format!("Token validation failed: {}", e),
                    })
                    .await
                {
                    warn!("Failed to send token expired event: {}", event_err);
                }

                // Return with clear message
                Err(anyhow!(
                    "Token validation failed: {}. Need to re-authenticate",
                    e
                ))
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
