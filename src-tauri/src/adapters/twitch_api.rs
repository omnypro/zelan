use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tracing::{debug, info, warn};
use twitch_api::helix::{channels::ChannelInformation, streams::Stream};
use twitch_oauth2::{ClientId, UserToken};

use crate::adapters::{
    common::{AdapterError, BackoffStrategy, RetryOptions, TraceHelper},
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

        // Implement direct sequential retry logic
        let operation_name = "fetch_channel_info";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": retry_options.max_attempts,
                "backoff": format!("{:?}", retry_options.backoff),
                "broadcaster_id": broadcaster_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

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
                    "broadcaster_id": broadcaster_id,
                })),
            )
            .await;

            // Get client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => id,
                Err(e) => {
                    let error = AdapterError::from_anyhow_error(
                        "config",
                        format!("Failed to get client ID: {}", e),
                        e,
                    );

                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "broadcaster_id": broadcaster_id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "broadcaster_id": broadcaster_id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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

            // Check status
            if !(response.status() >= 200 && response.status() < 300) {
                warn!(
                    status = response.status(),
                    body = response.body(),
                    attempt = attempt,
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
                        "attempt": attempt,
                        "max_attempts": retry_options.max_attempts,
                        "error": error.to_string(),
                        "status_code": response.status(),
                        "broadcaster_id": broadcaster_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

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

            // Parse JSON from the response body
            let response_json: Value = match serde_json::from_str(response.body()) {
                Ok(json) => json,
                Err(e) => {
                    let error =
                        AdapterError::api_with_source("Failed to parse Twitch API response", e);

                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "broadcaster_id": broadcaster_id,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            result = Some(Ok(Some(channel_info)));
                            break;
                        }
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
                                    "attempt": attempt,
                                    "max_attempts": retry_options.max_attempts,
                                    "error": error.to_string(),
                                    "broadcaster_id": broadcaster_id,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

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
                    }
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
                    "attempt": attempt,
                    "broadcaster_id": broadcaster_id,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            result = Some(Ok(None));
            break;
        }

        // Process the final result
        match result {
            Some(ok_result) => ok_result,
            None => {
                // Record final failure
                let error_msg = match last_error {
                    Some(ref e) => e.to_string(),
                    None => "Unknown error during channel info fetch".to_string(),
                };

                TraceHelper::record_adapter_operation(
                    "twitch",
                    &format!("{}_all_attempts_failed", operation_name),
                    Some(serde_json::json!({
                        "max_attempts": retry_options.max_attempts,
                        "error": error_msg,
                        "broadcaster_id": broadcaster_id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                // Return the last error
                Err(last_error.unwrap_or_else(|| {
                    AdapterError::api("Failed to fetch channel info after all retry attempts")
                }))
            }
        }
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

        // Implement direct sequential retry logic
        let operation_name = "fetch_stream_info";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": retry_options.max_attempts,
                "backoff": format!("{:?}", retry_options.backoff),
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
                    "user_id": user_id,
                    "user_login": user_login,
                })),
            )
            .await;

            // Get client ID from environment
            let client_id = match get_client_id() {
                Ok(id) => id,
                Err(e) => {
                    let error = AdapterError::from_anyhow_error(
                        "config",
                        format!("Failed to get client ID: {}", e),
                        e,
                    );

                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "user_id": user_id,
                            "user_login": user_login,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "user_id": user_id,
                            "user_login": user_login,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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

            // Check status
            if !(response.status() >= 200 && response.status() < 300) {
                warn!(
                    status = response.status(),
                    body = response.body(),
                    attempt = attempt,
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
                        "attempt": attempt,
                        "max_attempts": retry_options.max_attempts,
                        "error": error.to_string(),
                        "status_code": response.status(),
                        "user_id": user_id,
                        "user_login": user_login,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

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

            // Parse JSON from the response body
            let response_json: Value = match serde_json::from_str(response.body()) {
                Ok(json) => json,
                Err(e) => {
                    let error =
                        AdapterError::api_with_source("Failed to parse Twitch API response", e);

                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "max_attempts": retry_options.max_attempts,
                            "error": error.to_string(),
                            "user_id": user_id,
                            "user_login": user_login,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

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

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "twitch",
                                &format!("{}_success", operation_name),
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "user_id": user_id,
                                    "user_login": user_login,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            result = Some(Ok(Some(stream_info)));
                            break;
                        }
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
                                    "attempt": attempt,
                                    "max_attempts": retry_options.max_attempts,
                                    "error": error.to_string(),
                                    "user_id": user_id,
                                    "user_login": user_login,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

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
                    }
                }
            }

            // No stream found (user is offline) - not an error
            debug!(
                user_id = ?user_id,
                user_login = ?user_login,
                "No stream information found (user likely offline)"
            );

            // Record success in trace with no data
            TraceHelper::record_adapter_operation(
                "twitch",
                &format!("{}_success_no_data", operation_name),
                Some(serde_json::json!({
                    "attempt": attempt,
                    "user_id": user_id,
                    "user_login": user_login,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            result = Some(Ok(None));
            break;
        }

        // Process the final result
        match result {
            Some(ok_result) => ok_result,
            None => {
                // Record final failure
                let error_msg = match last_error {
                    Some(ref e) => e.to_string(),
                    None => "Unknown error during stream info fetch".to_string(),
                };

                TraceHelper::record_adapter_operation(
                    "twitch",
                    &format!("{}_all_attempts_failed", operation_name),
                    Some(serde_json::json!({
                        "max_attempts": retry_options.max_attempts,
                        "error": error_msg,
                        "user_id": user_id,
                        "user_login": user_login,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                // Return the last error
                Err(last_error.unwrap_or_else(|| {
                    AdapterError::api("Failed to fetch stream info after all retry attempts")
                }))
            }
        }
    }

    // Add more API methods as needed...
}

#[cfg(test)]
mod tests {
    // Tests for this module have been moved to src/adapters/tests/twitch_api_test.rs
    pub use crate::adapters::tests::twitch_api_test::*;
}
