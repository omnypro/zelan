use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tracing::{debug, info, warn};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::{ClientId, UserToken};

use crate::adapters::{
    common::{AdapterError, TraceHelper},
    http_client::{HttpClient, ReqwestHttpClient},
};

/// Environment variable name for Twitch Client ID
pub(crate) const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

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
            http_client: Arc::clone(&self.http_client),
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

        // Set up for retry with our helper
        let operation_name = "fetch_channel_info";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": 3, // Default max attempts
                "broadcaster_id": broadcaster_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Use our new retry helper
        use crate::common::retry::with_retry;

        // Clone token for use in retry operation
        let token_clone = token.clone();
        let self_clone = self.clone();
        let broadcaster_id_clone = broadcaster_id.to_string();

        // Use the retry helper
        let result = with_retry(
            || {
                // Create clones for each invocation of the closure
                let token = token_clone.clone();
                let self_clone = self_clone.clone(); // Clone the whole struct
                let broadcaster_id = broadcaster_id_clone.clone();

                Box::pin(async move {
                    // Record attempt
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_attempt", operation_name),
                        Some(serde_json::json!({
                            "broadcaster_id": broadcaster_id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Get client ID from environment - using match instead of map_err
                    let client_id = match get_client_id() {
                        Ok(id) => id,
                        Err(e) => {
                            // Record failure and convert to AdapterError
                            let error = AdapterError::from_anyhow_error(
                                "config",
                                format!("Failed to get client ID: {}", e),
                                e,
                            );

                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
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

                    // Make the request using our abstracted HTTP client - using match instead of map_err
                    let response = match self_clone.http_client.get(&url, headers).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            // Log and convert to AdapterError
                            warn!(
                                error = %e,
                                "HTTP request failed when fetching channel info"
                            );

                            let error = AdapterError::from_anyhow_error(
                                "connection",
                                format!("Failed to connect to Twitch API: {}", e),
                                anyhow::anyhow!(e),
                            );

                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Check status
                    if !(response.status() >= 200 && response.status() < 300) {
                        warn!(
                            status = response.status(),
                            body = response.body(),
                            "Received error status from Twitch API"
                        );

                        let error = AdapterError::from_anyhow_error_with_status(
                            format!("Twitch API error: {}", response.body()),
                            response.status(),
                            anyhow!("API request failed with status {}", response.status()),
                        );

                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "error": error.to_string(),
                                "status_code": response.status(),
                                "broadcaster_id": broadcaster_id,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        return Err(error);
                    }

                    // Parse JSON from the response body
                    let response_json: Value = match serde_json::from_str(response.body()) {
                        Ok(json) => json,
                        Err(e) => {
                            let error = AdapterError::api_with_source(
                                "Failed to parse Twitch API response",
                                e,
                            );

                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Check if we have data
                    if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
                        if let Some(channel) = data.first() {
                            // Convert to ChannelInformation
                            let channel_info = match serde_json::from_value(channel.clone()) {
                                Ok(info) => info,
                                Err(e) => {
                                    let error = AdapterError::from_anyhow_error(
                                        "api",
                                        "Failed to deserialize channel information",
                                        anyhow::anyhow!(e),
                                    );

                                    // Record failure
                                    TraceHelper::record_adapter_operation(
                                        "twitch",
                                        &format!("{}_failure", operation_name),
                                        Some(serde_json::json!({
                                            "error": error.to_string(),
                                            "broadcaster_id": broadcaster_id,
                                            "timestamp": chrono::Utc::now().to_rfc3339(),
                                        })),
                                    )
                                    .await;

                                    return Err(error);
                                }
                            };

                            info!(
                                broadcaster_id = %broadcaster_id,
                                "Successfully fetched channel information"
                            );

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Ok(Some(channel_info));
                        }
                    }

                    // No channel found - not an error, just no data
                    debug!(
                        "No channel information found for broadcaster ID: {}",
                        broadcaster_id
                    );

                    // Record success in trace with no data
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_success_no_data", operation_name),
                        Some(serde_json::json!({
                            "broadcaster_id": broadcaster_id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    Ok(None)
                })
            },
            3, // max attempts
            operation_name,
        )
        .await;

        // Return the result
        result
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

        // Set up for retry with our helper
        let operation_name = "fetch_stream_info";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": 3, // Default max attempts
                "user_id": user_id,
                "user_login": user_login,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Need either user_id or user_login - verify this up front
        let query_param = match (user_id, user_login) {
            (Some(id), _) => format!("user_id={}", id),
            (_, Some(login)) => format!("user_login={}", login),
            _ => {
                // This is a parameter validation error, not a retry-able error
                let error = AdapterError::api("Either user_id or user_login must be provided");

                // Record the validation error
                TraceHelper::record_adapter_operation(
                    "twitch",
                    &format!("{}_validation_error", operation_name),
                    Some(serde_json::json!({
                        "error": error.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                return Err(error);
            }
        };

        // Use our new retry helper
        use crate::common::retry::with_retry;

        // Clone values for use in retry operation
        let token_clone = token.clone();
        let self_clone = self.clone();
        let query_param_clone = query_param.clone();

        // Store user_id and user_login as owned Strings inside the closure
        let user_id_owned = user_id.map(|s| s.to_string());
        let user_login_owned = user_login.map(|s| s.to_string());

        // Use the retry helper
        with_retry(
            || {
                // Create clones for each invocation of the closure
                let token = token_clone.clone();
                let self_clone = self_clone.clone();
                let query_param = query_param_clone.clone();
                
                // Create owned versions for this iteration of the closure
                let user_id_owned = user_id_owned.clone();
                let user_login_owned = user_login_owned.clone();
                
                Box::pin(async move {
                    // Inside the async block, get references when needed
                    let user_id_ref = user_id_owned.as_deref();
                    let user_login_ref = user_login_owned.as_deref();
                    // Record attempt
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_attempt", operation_name),
                        Some(serde_json::json!({
                            "user_id": user_id_ref,
                            "user_login": user_login_ref,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
    
                    // Get client ID from environment - using match instead of map_err
                    let client_id = match get_client_id() {
                        Ok(id) => id,
                        Err(e) => {
                            // Record failure and convert to AdapterError
                            let error = AdapterError::from_anyhow_error(
                                "config",
                                format!("Failed to get client ID: {}", e),
                                e,
                            );
                            
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "user_id": user_id_ref,
                                    "user_login": user_login_ref,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            return Err(error);
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
    
                    // Make the request using our abstracted HTTP client - using match instead of map_err
                    let response = match self_clone.http_client.get(&url, headers).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            // Log and convert to AdapterError
                            warn!(
                                error = %e,
                                "HTTP request failed when fetching stream info"
                            );
                            
                            let error = AdapterError::from_anyhow_error(
                                "connection",
                                format!("Failed to connect to Twitch API: {}", e),
                                anyhow::anyhow!(e),
                            );
                            
                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "user_id": user_id_ref,
                                    "user_login": user_login_ref,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            return Err(error);
                        }
                    };
    
                    // Check status
                    if !(response.status() >= 200 && response.status() < 300) {
                        warn!(
                            status = response.status(),
                            body = response.body(),
                            "Received error status from Twitch API"
                        );
    
                        let error = AdapterError::from_anyhow_error_with_status(
                            format!("Twitch API error: {}", response.body()),
                            response.status(),
                            anyhow!("API request failed with status {}", response.status()),
                        );
    
                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "error": error.to_string(),
                                "status_code": response.status(),
                                "user_id": user_id_ref,
                                "user_login": user_login_ref, 
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;
    
                        return Err(error);
                    }
    
                    // Parse JSON from the response body - using match instead of map_err
                    let response_json: Value = match serde_json::from_str(response.body()) {
                        Ok(json) => json,
                        Err(e) => {
                            let error = AdapterError::api_with_source(
                                "Failed to parse Twitch API response", e
                            );
                            
                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "user_id": user_id_ref,
                                    "user_login": user_login_ref,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
                            
                            return Err(error);
                        }
                    };
    
                    // Check if we have data
                    if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
                        if let Some(stream) = data.first() {
                            // Convert to Stream - using match instead of map_err
                            let stream_info = match serde_json::from_value(stream.clone()) {
                                Ok(info) => info,
                                Err(e) => {
                                    let error = AdapterError::from_anyhow_error(
                                        "api",
                                        "Failed to deserialize stream information",
                                        anyhow::anyhow!(e),
                                    );
                                    
                                    // Record failure
                                    TraceHelper::record_adapter_operation(
                                        "twitch",
                                        &format!("{}_failure", operation_name),
                                        Some(serde_json::json!({
                                            "error": error.to_string(),
                                            "user_id": user_id_ref,
                                            "user_login": user_login_ref,
                                            "timestamp": chrono::Utc::now().to_rfc3339(),
                                        })),
                                    ).await;
                                    
                                    return Err(error);
                                }
                            };
                                
                            info!(
                                user_id = ?user_id_ref,
                                user_login = ?user_login_ref,
                                "Successfully fetched stream information"
                            );
    
                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "user_id": user_id_ref,
                                    "user_login": user_login_ref,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            ).await;
    
                            return Ok(Some(stream_info));
                        }
                    }
    
                    // No stream found (user is offline) - not an error, just no data
                    debug!(
                        "No stream information found for user_id: {:?}, user_login: {:?} (user likely offline)",
                        user_id_ref, user_login_ref
                    );
    
                    // Record success in trace with no data
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_success_no_data", operation_name),
                        Some(serde_json::json!({
                            "user_id": user_id_ref,
                            "user_login": user_login_ref,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;
    
                    Ok(None)
                })
            },
            3, // max attempts
            operation_name
        ).await
    }

    // Add more API methods as needed...
}

#[cfg(test)]
mod tests {
    pub use crate::adapters::tests::twitch_api_test::*;
}
