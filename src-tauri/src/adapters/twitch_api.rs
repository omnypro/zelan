use anyhow::{anyhow, Result};
use serde_json::Value;
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::{ClientId, UserToken};
use tracing::debug;
use std::env;

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

/// Client for Twitch API requests
pub struct TwitchApiClient {
    /// HTTP client for API requests
    http_client: reqwest::Client,
}

impl Clone for TwitchApiClient {
    fn clone(&self) -> Self {
        // Create a new client since reqwest::Client doesn't implement Clone
        Self {
            http_client: reqwest::Client::new(),
        }
    }
}

impl TwitchApiClient {
    /// Create a new API client
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
    
    /// Fetch channel information
    pub async fn fetch_channel_info(
        &self,
        token: &UserToken,
        broadcaster_id: &str,
    ) -> Result<Option<ChannelInformation>> {
        debug!("Fetching channel info for broadcaster ID: {}", broadcaster_id);
        
        // Get client ID from environment
        let client_id = get_client_id()?;
        
        // Define the API endpoint
        let url = format!(
            "https://api.twitch.tv/helix/channels?broadcaster_id={}",
            broadcaster_id
        );
        
        // Make the request
        let response = self.http_client
            .get(&url)
            .header("Client-ID", client_id.as_str())
            .header("Authorization", format!("Bearer {}", token.access_token.secret()))
            .send()
            .await?;
        
        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!(
                "Failed to fetch channel info: HTTP {} - {}",
                status,
                error_text
            ));
        }
        
        // Parse response
        let response_json: Value = response.json().await?;
        
        // Check if we have data
        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(channel) = data.first() {
                // Convert to ChannelInformation
                let channel_info = serde_json::from_value(channel.clone())?;
                return Ok(Some(channel_info));
            }
        }
        
        // No channel found
        Ok(None)
    }
    
    /// Fetch stream information
    pub async fn fetch_stream_info(
        &self,
        token: &UserToken,
        user_id: Option<&str>,
        user_login: Option<&str>,
    ) -> Result<Option<Stream>> {
        debug!(
            "Fetching stream info for user_id: {:?}, user_login: {:?}",
            user_id, user_login
        );
        
        // Get client ID from environment
        let client_id = get_client_id()?;
        
        // Need either user_id or user_login
        let query_param = match (user_id, user_login) {
            (Some(id), _) => format!("user_id={}", id),
            (_, Some(login)) => format!("user_login={}", login),
            _ => return Err(anyhow!("Either user_id or user_login must be provided")),
        };
        
        // Define the API endpoint
        let url = format!(
            "https://api.twitch.tv/helix/streams?{}",
            query_param
        );
        
        // Make the request
        let response = self.http_client
            .get(&url)
            .header("Client-ID", client_id.as_str())
            .header("Authorization", format!("Bearer {}", token.access_token.secret()))
            .send()
            .await?;
        
        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!(
                "Failed to fetch stream info: HTTP {} - {}",
                status,
                error_text
            ));
        }
        
        // Parse response
        let response_json: Value = response.json().await?;
        
        // Check if we have data
        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(stream) = data.first() {
                // Convert to Stream
                let stream_info = serde_json::from_value(stream.clone())?;
                return Ok(Some(stream_info));
            }
        }
        
        // No stream found (user is offline)
        Ok(None)
    }
    
    // Add more API methods as needed...
}
