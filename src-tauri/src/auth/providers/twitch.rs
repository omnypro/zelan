use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::auth::providers::{AuthenticationFlow, AuthFlowStatus, AuthProvider};
use crate::auth::token::TokenData;

// Constants for Twitch auth
const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TWITCH_TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const TWITCH_DEVICE_CODE_URL: &str = "https://id.twitch.tv/oauth2/device";
const TWITCH_VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";
const TWITCH_REVOKE_URL: &str = "https://id.twitch.tv/oauth2/revoke";
const DEFAULT_SCOPES: &[&str] = &[
    "channel:read:subscriptions",
    "channel:read:stream_key",
    "bits:read",
    "moderation:read",
    "channel:read:redemptions",
    "user:read:email",
];

// Response type for device code flow initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

// Response type for token requests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: Option<Vec<String>>,
    token_type: String,
}

// Response for validation requests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidateResponse {
    client_id: String,
    login: String,
    scopes: Vec<String>,
    user_id: String,
    expires_in: u64,
}

// Error response from Twitch
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorResponse {
    status: u16,
    message: String,
}

// Structure to track active device code flows
#[derive(Debug, Clone)]
struct DeviceCodeFlow {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_at: chrono::DateTime<Utc>,
    poll_interval: Duration,
}

/// Twitch authentication provider
pub struct TwitchAuthProvider {
    /// HTTP client for API requests
    client: Client,
    /// Client ID for Twitch API
    client_id: Option<String>,
    /// Active device code flows by flow ID
    active_flows: Arc<RwLock<HashMap<String, DeviceCodeFlow>>>,
}

impl TwitchAuthProvider {
    /// Create a new Twitch auth provider
    pub fn new() -> Self {
        // Create a client with reasonable defaults
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            client_id: None,
            active_flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Set a client ID directly (for testing)
    pub fn with_client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(client_id.to_string());
        self
    }
    
    /// Get the client ID from environment or stored value
    async fn get_client_id(&self) -> Result<String> {
        // Return stored client ID if available
        if let Some(client_id) = &self.client_id {
            return Ok(client_id.clone());
        }
        
        // Otherwise, get from environment
        match env::var("TWITCH_CLIENT_ID") {
            Ok(id) if !id.is_empty() => Ok(id),
            _ => Err(anyhow!("TWITCH_CLIENT_ID environment variable not set")),
        }
    }
    
    /// Create a device code for authentication
    async fn create_device_code(&self, client_id: String) -> Result<DeviceCodeResponse> {
        let scopes = DEFAULT_SCOPES.join(" ");
        
        let params = [
            ("client_id", client_id.as_str()),
            ("scope", &scopes),
        ];
        
        let response = self.client
            .post(TWITCH_DEVICE_CODE_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to request device code")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to get device code: HTTP {}: {}", status, error_text));
        }
        
        let device_code_response = response.json::<DeviceCodeResponse>().await
            .context("Failed to parse device code response")?;
            
        debug!(
            user_code = %device_code_response.user_code,
            verification_uri = %device_code_response.verification_uri,
            expires_in = device_code_response.expires_in,
            "Received device code for Twitch authentication"
        );
        
        Ok(device_code_response)
    }
    
    /// Poll for token using device code
    async fn poll_device_code(&self, device_code: &str) -> Result<TokenResponse> {
        let client_id = self.get_client_id().await?;
        
        let params = [
            ("client_id", client_id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];
        
        let response = self.client
            .post(TWITCH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to poll for token")?;
            
        match response.status() {
            StatusCode::OK => {
                let token_response = response.json::<TokenResponse>().await
                    .context("Failed to parse token response")?;
                Ok(token_response)
            },
            StatusCode::BAD_REQUEST => {
                // This might be "authorization_pending" which is normal during polling
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                if error_text.contains("authorization_pending") {
                    Err(anyhow!("Authorization pending"))
                } else if error_text.contains("expired") {
                    Err(anyhow!("Device code expired"))
                } else {
                    Err(anyhow!("Bad request during polling: {}", error_text))
                }
            },
            status => {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to poll for token: HTTP {}: {}", status, error_text))
            }
        }
    }
    
    /// Convert a TokenResponse to TokenData
    fn token_response_to_token_data(&self, response: TokenResponse) -> TokenData {
        // Calculate expiration
        let expires_in_secs = response.expires_in as i64;
        let expiration = Utc::now() + chrono::Duration::seconds(expires_in_secs);
        
        // Parse scopes, falling back to default if not provided
        let scope = response.scope.unwrap_or_else(|| 
            DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
        );
        
        // Create token data
        TokenData::new(
            response.access_token,
            response.refresh_token,
            Some(expiration),
            "twitch".to_string(),
            scope,
            None, // We'll get user_id from validation
        )
    }
    
    /// Refresh a token using the refresh token
    async fn do_refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let client_id = self.get_client_id().await?;
        
        let params = [
            ("client_id", client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        
        let response = self.client
            .post(TWITCH_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to refresh token")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // Special handling for invalid refresh token
            if status == StatusCode::BAD_REQUEST && error_text.contains("Invalid refresh token") {
                return Err(anyhow!("Invalid refresh token - user must re-authenticate"));
            }
            
            return Err(anyhow!("Failed to refresh token: HTTP {}: {}", status, error_text));
        }
        
        let token_response = response.json::<TokenResponse>().await
            .context("Failed to parse token response")?;
            
        debug!("Successfully refreshed Twitch token");
        
        Ok(token_response)
    }
    
    /// Validate a token with Twitch
    async fn do_validate_token(&self, access_token: &str) -> Result<ValidateResponse> {
        // Twitch uses a GET request with the token in the Authorization header
        let response = self.client
            .get(TWITCH_VALIDATE_URL)
            .header("Authorization", format!("OAuth {}", access_token))
            .send()
            .await
            .context("Failed to validate token")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to validate token: HTTP {}: {}", status, error_text));
        }
        
        let validate_response = response.json::<ValidateResponse>().await
            .context("Failed to parse validation response")?;
            
        debug!(
            user_id = %validate_response.user_id,
            login = %validate_response.login,
            expires_in = validate_response.expires_in,
            "Successfully validated Twitch token"
        );
        
        Ok(validate_response)
    }
}

#[async_trait]
impl AuthProvider for TwitchAuthProvider {
    fn name(&self) -> &str {
        "twitch"
    }
    
    async fn authenticate(&self) -> Result<AuthenticationFlow> {
        // Get client ID from environment
        let client_id = self.get_client_id().await?;
        
        // Create a device code flow
        let device_code_response = self.create_device_code(client_id).await?;
        
        // Store the flow in active_flows
        let flow_id = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::seconds(device_code_response.expires_in as i64);
        let poll_interval = Duration::from_secs(device_code_response.interval);
        
        let flow = DeviceCodeFlow {
            device_code: device_code_response.device_code.clone(),
            user_code: device_code_response.user_code.clone(),
            verification_uri: device_code_response.verification_uri.clone(),
            expires_at,
            poll_interval,
        };
        
        self.active_flows.write().await.insert(flow_id.clone(), flow);
        
        // Return the flow info for the frontend to display
        Ok(AuthenticationFlow::DeviceCode {
            verification_uri: device_code_response.verification_uri,
            user_code: device_code_response.user_code,
            expires_in: device_code_response.expires_in,
            poll_interval: device_code_response.interval,
            provider: "twitch".to_string(),
        })
    }
    
    async fn refresh_token(&self, token: TokenData) -> Result<TokenData> {
        let refresh_token = token.refresh_token.as_deref().ok_or_else(|| {
            anyhow!("No refresh token available")
        })?;
        
        // Perform the refresh
        let token_response = self.do_refresh_token(refresh_token).await?;
        
        // Convert response to token data
        let mut new_token = self.token_response_to_token_data(token_response);
        
        // Preserve user_id from the previous token if available
        if let Some(user_id) = token.user_id {
            new_token.user_id = Some(user_id);
        } else {
            // If we don't have a user_id, validate the token to get it
            match self.do_validate_token(&new_token.access_token).await {
                Ok(validate_response) => {
                    new_token.user_id = Some(validate_response.user_id);
                }
                Err(e) => {
                    warn!("Failed to validate new token after refresh: {}", e);
                    // Continue anyway, we have a new token
                }
            }
        }
        
        // Preserve metadata from the previous token
        for (key, value) in token.metadata {
            new_token.metadata.insert(key, value);
        }
        
        Ok(new_token)
    }
    
    async fn validate_token(&self, mut token: TokenData) -> Result<TokenData> {
        // Validate with Twitch
        let validate_response = self.do_validate_token(&token.access_token).await?;
        
        // Update token with validated information
        token.user_id = Some(validate_response.user_id);
        
        // Update expiration based on validation response
        let expires_in_secs = validate_response.expires_in as i64;
        token.expiration = Some(Utc::now() + chrono::Duration::seconds(expires_in_secs));
        
        Ok(token)
    }
    
    async fn check_flow_status(&self, flow_id: &str) -> Result<AuthFlowStatus> {
        let flow = {
            let flows = self.active_flows.read().await;
            match flows.get(flow_id) {
                Some(flow) => flow.clone(),
                None => return Err(anyhow!("No active flow with ID {}", flow_id)),
            }
        };
        
        // Check if the flow has expired
        if Utc::now() > flow.expires_at {
            // Remove expired flow
            self.active_flows.write().await.remove(flow_id);
            return Ok(AuthFlowStatus::Expired);
        }
        
        // Try to get a token
        match self.poll_device_code(&flow.device_code).await {
            Ok(token_response) => {
                // Success! We got a token
                let mut token = self.token_response_to_token_data(token_response);
                
                // Validate to get user_id
                match self.do_validate_token(&token.access_token).await {
                    Ok(validate_response) => {
                        token.user_id = Some(validate_response.user_id.clone());
                        
                        // Store user information in metadata
                        token.set_metadata("username", validate_response.login)?;
                        token.set_metadata("client_id", validate_response.client_id)?;
                    }
                    Err(e) => {
                        warn!("Failed to validate new token: {}", e);
                        // Continue anyway, we have a new token
                    }
                }
                
                // Remove the flow
                self.active_flows.write().await.remove(flow_id);
                
                Ok(AuthFlowStatus::Completed { token })
            }
            Err(e) => {
                // If authorization is pending, that's normal during polling
                if e.to_string().contains("Authorization pending") {
                    Ok(AuthFlowStatus::Pending)
                } else if e.to_string().contains("expired") {
                    // Remove expired flow
                    self.active_flows.write().await.remove(flow_id);
                    Ok(AuthFlowStatus::Expired)
                } else {
                    // Other error
                    Ok(AuthFlowStatus::Failed { 
                        reason: e.to_string() 
                    })
                }
            }
        }
    }
    
    async fn revoke_token(&self, token: TokenData) -> Result<()> {
        let client_id = self.get_client_id().await?;
        
        let params = [
            ("client_id", client_id.as_str()),
            ("token", token.access_token.as_str()),
        ];
        
        let response = self.client
            .post(TWITCH_REVOKE_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to revoke token")?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to revoke token: HTTP {}: {}", status, error_text));
        }
        
        debug!("Successfully revoked Twitch token");
        
        Ok(())
    }
}