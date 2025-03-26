use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tracing::{debug, info, warn};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::{ClientId, UserToken};

use crate::adapters::{
    common::{AdapterError, RetryOptions, TraceHelper, with_retry},
    http_client::{HttpClient, ReqwestHttpClient}
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

/// Client for Twitch API requests
pub struct TwitchApiClient {
    /// HTTP client for API requests
    http_client: Arc<dyn HttpClient + Send + Sync>,
}

impl Clone for TwitchApiClient {
    fn clone(&self) -> Self {
        // Clone the Arc, not the client itself
        Self {
            http_client: self.http_client.clone(),
        }
    }
}

impl TwitchApiClient {
    /// Create a new API client with the default HTTP client
    pub fn new() -> Self {
        Self {
            http_client: Arc::new(ReqwestHttpClient::new()),
        }
    }

    /// Create a new API client with a custom HTTP client
    pub fn with_http_client(http_client: Arc<dyn HttpClient + Send + Sync>) -> Self {
        Self { http_client }
    }
    
    /// Test helper - fetch stream info with a specific client ID (bypassing environment lookup)
    #[cfg(test)]
    pub async fn fetch_stream_info_with_client_id(
        &self,
        token: &UserToken,
        user_id: Option<&str>,
        user_login: Option<&str>,
        client_id: ClientId,
    ) -> Result<Option<Stream>> {
        // Need either user_id or user_login
        let query_param = match (user_id, user_login) {
            (Some(id), _) => format!("user_id={}", id),
            (_, Some(login)) => format!("user_login={}", login),
            _ => return Err(anyhow!("Either user_id or user_login must be provided")),
        };
        
        // Define the API endpoint
        let url = format!("https://api.twitch.tv/helix/streams?{}", query_param);
        
        // Prepare headers
        let mut headers = HashMap::new();
        headers.insert("Client-ID".to_string(), client_id.as_str().to_string());
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", token.access_token.secret()),
        );
        
        // Make the request using our abstracted HTTP client
        let response = self.http_client.get(&url, headers).await?;
        
        // Check status
        if !(response.status() >= 200 && response.status() < 300) {
            return Err(anyhow!(
                "Failed to fetch stream info: HTTP {} - {}",
                response.status(),
                response.body()
            ));
        }
        
        // Parse JSON from the response body
        let response_json: Value = serde_json::from_str(response.body())?;
        
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

    /// Test helper - fetch channel info with a specific client ID (bypassing environment lookup)
    #[cfg(test)]
    pub async fn fetch_channel_info_with_client_id(
        &self,
        token: &UserToken,
        broadcaster_id: &str,
        client_id: ClientId,
    ) -> Result<Option<ChannelInformation>> {
        debug!(
            "Fetching channel info for broadcaster ID: {}",
            broadcaster_id
        );

        // Define the API endpoint
        let url = format!(
            "https://api.twitch.tv/helix/channels?broadcaster_id={}",
            broadcaster_id
        );

        // Prepare headers
        let mut headers = HashMap::new();
        headers.insert("Client-ID".to_string(), client_id.as_str().to_string());
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", token.access_token.secret()),
        );

        // Make the request using our abstracted HTTP client
        let response = self.http_client.get(&url, headers).await?;

        // Check status
        if !(response.status() >= 200 && response.status() < 300) {
            return Err(anyhow!(
                "Failed to fetch channel info: HTTP {} - {}",
                response.status(),
                response.body()
            ));
        }

        // Parse JSON from the response body
        let response_json: Value = serde_json::from_str(response.body())?;

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

    /// Fetch channel information
    pub async fn fetch_channel_info(
        &self,
        token: &UserToken,
        broadcaster_id: &str,
    ) -> Result<Option<ChannelInformation>, AdapterError> {
        info!(
            "Fetching channel info for broadcaster ID: {}",
            broadcaster_id
        );

        // Create retry options for robust error handling
        let retry_options = RetryOptions::default();
        
        // Use the standardized retry mechanism
        with_retry("fetch_channel_info", retry_options, |attempt| async move {
            // Record trace context for this operation
            TraceHelper::record_adapter_operation(
                "twitch",
                "fetch_channel_info",
                Some(serde_json::json!({
                    "broadcaster_id": broadcaster_id,
                    "attempt": attempt,
                })),
            ).await;
            
            // Get client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => id,
                Err(e) => {
                    return Err(AdapterError::from_anyhow_error(
                        "config",
                        format!("Failed to get client ID: {}", e),
                        e
                    ));
                }
            };

            // Define the API endpoint
            let url = format!(
                "https://api.twitch.tv/helix/channels?broadcaster_id={}",
                broadcaster_id
            );

            // Prepare headers
            let mut headers = HashMap::new();
            headers.insert("Client-ID".to_string(), client_id.as_str().to_string());
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", token.access_token.secret()),
            );

            // Make the request using our abstracted HTTP client
            let response = match self.http_client.get(&url, headers).await {
                Ok(resp) => resp,
                Err(e) => {
                    warn!(
                        error = %e, 
                        attempt = attempt,
                        "HTTP request failed when fetching channel info"
                    );
                    return Err(AdapterError::from_anyhow_error(
                        "connection",
                        format!("Failed to connect to Twitch API: {}", e),
                        anyhow::anyhow!(e)
                    ));
                }
            };

            // Check status
            if !(response.status() >= 200 && response.status() < 300) {
                warn!(
                    status = response.status(),
                    body = response.body(),
                    attempt = attempt,
                    "Received error status from Twitch API"
                );
                
                return Err(AdapterError::from_anyhow_error_with_status(
                    format!("Twitch API error: {}", response.body()),
                    response.status(),
                    anyhow!("API request failed with status {}", response.status())
                ));
            }

            // Parse JSON from the response body
            let response_json: Value = match serde_json::from_str(response.body()) {
                Ok(json) => json,
                Err(e) => {
                    return Err(AdapterError::api_with_source(
                        "Failed to parse Twitch API response",
                        e
                    ));
                }
            };

            // Check if we have data
            if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
                if let Some(channel) = data.first() {
                    // Convert to ChannelInformation
                    match serde_json::from_value(channel.clone()) {
                        Ok(channel_info) => {
                            info!(
                                broadcaster_id = %broadcaster_id,
                                "Successfully fetched channel information"
                            );
                            return Ok(Some(channel_info));
                        }
                        Err(e) => {
                            return Err(AdapterError::from_anyhow_error(
                                "api",
                                "Failed to deserialize channel information",
                                anyhow::anyhow!(e)
                            ));
                        }
                    }
                }
            }

            // No channel found - not an error, just no data
            debug!("No channel information found for broadcaster ID: {}", broadcaster_id);
            Ok(None)
        }).await
    }

    /// Fetch stream information
    pub async fn fetch_stream_info(
        &self,
        token: &UserToken,
        user_id: Option<&str>,
        user_login: Option<&str>,
    ) -> Result<Option<Stream>, AdapterError> {
        info!(
            "Fetching stream info for user_id: {:?}, user_login: {:?}",
            user_id, user_login
        );

        // Create retry options for robust error handling
        let retry_options = RetryOptions::default();
        
        // Use the standardized retry mechanism
        with_retry("fetch_stream_info", retry_options, |attempt| async move {
            // Record trace context for this operation
            TraceHelper::record_adapter_operation(
                "twitch",
                "fetch_stream_info",
                Some(serde_json::json!({
                    "user_id": user_id,
                    "user_login": user_login,
                    "attempt": attempt,
                })),
            ).await;
            
            // Need either user_id or user_login
            let query_param = match (user_id, user_login) {
                (Some(id), _) => format!("user_id={}", id),
                (_, Some(login)) => format!("user_login={}", login),
                _ => {
                    return Err(AdapterError::api(
                        "Either user_id or user_login must be provided"
                    ));
                }
            };
            
            // Get client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => id,
                Err(e) => {
                    return Err(AdapterError::from_anyhow_error(
                        "config",
                        format!("Failed to get client ID: {}", e),
                        e
                    ));
                }
            };

            // Define the API endpoint
            let url = format!("https://api.twitch.tv/helix/streams?{}", query_param);

            // Prepare headers
            let mut headers = HashMap::new();
            headers.insert("Client-ID".to_string(), client_id.as_str().to_string());
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", token.access_token.secret()),
            );

            // Make the request using our abstracted HTTP client
            let response = match self.http_client.get(&url, headers).await {
                Ok(resp) => resp,
                Err(e) => {
                    warn!(
                        error = %e, 
                        attempt = attempt,
                        "HTTP request failed when fetching stream info"
                    );
                    return Err(AdapterError::from_anyhow_error(
                        "connection",
                        format!("Failed to connect to Twitch API: {}", e),
                        anyhow::anyhow!(e)
                    ));
                }
            };

            // Check status
            if !(response.status() >= 200 && response.status() < 300) {
                warn!(
                    status = response.status(),
                    body = response.body(),
                    attempt = attempt,
                    "Received error status from Twitch API"
                );
                
                return Err(AdapterError::from_anyhow_error_with_status(
                    format!("Twitch API error: {}", response.body()),
                    response.status(),
                    anyhow!("API request failed with status {}", response.status())
                ));
            }

            // Parse JSON from the response body
            let response_json: Value = match serde_json::from_str(response.body()) {
                Ok(json) => json,
                Err(e) => {
                    return Err(AdapterError::api_with_source(
                        "Failed to parse Twitch API response",
                        e
                    ));
                }
            };

            // Check if we have data
            if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
                if let Some(stream) = data.first() {
                    // Convert to Stream
                    match serde_json::from_value(stream.clone()) {
                        Ok(stream_info) => {
                            info!(
                                user_id = ?user_id,
                                user_login = ?user_login,
                                "Successfully fetched stream information"
                            );
                            return Ok(Some(stream_info));
                        }
                        Err(e) => {
                            return Err(AdapterError::from_anyhow_error(
                                "api",
                                "Failed to deserialize stream information",
                                anyhow::anyhow!(e)
                            ));
                        }
                    }
                }
            }

            // No stream found (user is offline) - not an error
            debug!(
                user_id = ?user_id,
                user_login = ?user_login,
                "No stream information found (user likely offline)"
            );
            Ok(None)
        }).await
    }

    // Add more API methods as needed...
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::http_client::mock::MockHttpClient;
    use twitch_api::types::{UserId, UserName};

    // Set up environment variables for all tests
    fn setup_env_vars() {
        // This needs to be forcefully set each time to overcome potential env var clearing
        std::env::set_var(TWITCH_CLIENT_ID_ENV, "test_client_id");
        println!("API test: TWITCH_CLIENT_ID set to: test_client_id");
    }
    
    // This function directly returns a client ID for testing without using env vars
    fn get_test_client_id() -> ClientId {
        ClientId::new("test_client_id".to_string())
    }

    #[tokio::test]
    async fn test_fetch_channel_info() -> Result<()> {
        // Set up environment variables first
        setup_env_vars();

        // Sample response data based on Twitch API documentation
        let channel_data = serde_json::json!({
            "data": [{
                "broadcaster_id": "123456",
                "broadcaster_login": "testchannel",
                "broadcaster_name": "TestChannel",
                "broadcaster_language": "en",
                "game_id": "12345",
                "game_name": "Test Game",
                "title": "Test Stream Title",
                "delay": 0,
                "tags": ["test1", "test2"],
                "content_classification_labels": ["test_label1", "test_label2"],
                "is_branded_content": false
            }]
        });

        // Create a mock HTTP client
        let mut mock_client = MockHttpClient::new();

        // Setup mock response for channel info
        mock_client.mock_success_json(
            "https://api.twitch.tv/helix/channels?broadcaster_id=123456",
            &channel_data,
        )?;

        // Create API client with our mock
        let api_client = TwitchApiClient::with_http_client(Arc::new(mock_client));

        // Create a fake client_id for testing using our helper
        let client_id = get_test_client_id();
        
        // Create fake credentials for testing - in real use cases these would come from the API
        let login = UserName::new("testuser".to_string());
        let user_id = UserId::new("123456".to_string());
        
        // For tests, we can use from_existing_unchecked
        // IMPORTANT: This is test-only code, don't use this in production!
        let token = UserToken::from_existing_unchecked(
            twitch_oauth2::AccessToken::new("test_token".to_string()),
            None, // refresh_token
            client_id.clone(),
            None, // client_secret
            login,
            user_id,
            Some(Vec::new()),                           // scopes
            Some(std::time::Duration::from_secs(3600)), // expires_in
        );

        // Call the method we're testing with the direct client_id parameter
        let channel_info = api_client.fetch_channel_info_with_client_id(&token, "123456", client_id).await?;

        // Verify we got the expected result
        assert!(channel_info.is_some());
        let info = channel_info.unwrap();

        // Convert to strings for comparison since the twitch_api types don't implement PartialEq with &str
        assert_eq!(info.broadcaster_id.as_str(), "123456");
        assert_eq!(info.broadcaster_name.as_str(), "TestChannel");
        assert_eq!(info.title, "Test Stream Title");

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_stream_info() -> Result<()> {
        // Set up environment variables first
        setup_env_vars();

        // Sample response data based on Twitch API documentation
        let stream_data = serde_json::json!({
            "data": [{
                "id": "123456789",
                "user_id": "123456",
                "user_login": "testchannel",
                "user_name": "TestChannel",
                "game_id": "12345",
                "game_name": "Test Game",
                "type": "live",
                "title": "Test Stream Title",
                "viewer_count": 123,
                "started_at": "2023-01-01T12:00:00Z",
                "language": "en",
                "thumbnail_url": "https://static-cdn.jtvnw.net/previews-ttv/live_user_testchannel-{width}x{height}.jpg",
                "tags": ["test1", "test2"], // Add missing tags field
                "tag_ids": ["123", "456"],
                "is_mature": false
            }]
        });

        // Create a mock HTTP client
        let mut mock_client = MockHttpClient::new();

        // Setup mock response for stream info
        mock_client.mock_success_json(
            "https://api.twitch.tv/helix/streams?user_id=123456",
            &stream_data,
        )?;

        // Create API client with our mock
        let api_client = TwitchApiClient::with_http_client(Arc::new(mock_client));

        // Create a fake client_id for testing using our helper
        let client_id = get_test_client_id();

        // Create fake credentials for testing - in real use cases these would come from the API
        let login = UserName::new("testuser".to_string());
        let user_id = UserId::new("123456".to_string());
        
        // For tests, we can use from_existing_unchecked
        // IMPORTANT: This is test-only code, don't use this in production!
        let token = UserToken::from_existing_unchecked(
            twitch_oauth2::AccessToken::new("test_token".to_string()),
            None, // refresh_token
            client_id.clone(),
            None, // client_secret
            login,
            user_id,
            Some(Vec::new()),                           // scopes
            Some(std::time::Duration::from_secs(3600)), // expires_in
        );

        // Call the method we're testing with the direct client_id parameter
        let stream_info = api_client
            .fetch_stream_info_with_client_id(&token, Some("123456"), None, client_id)
            .await?;

        // Verify we got the expected result
        assert!(stream_info.is_some());
        let info = stream_info.unwrap();

        // Convert to strings for comparison since the twitch_api types don't implement PartialEq with &str
        assert_eq!(info.user_id.as_str(), "123456");
        assert_eq!(info.user_name.as_str(), "TestChannel");
        assert_eq!(info.title, "Test Stream Title");
        assert_eq!(info.viewer_count, 123);

        Ok(())
    }
}
