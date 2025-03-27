//! Common utilities and types for adapters
//!
//! This module contains shared code used across different adapters,
//! helping to reduce duplication and standardize patterns.

use anyhow::Result;
use std::time::Duration;
use thiserror::Error;
use tracing::{error, info, warn};

/// Unified error type for adapter-related operations
#[derive(Error, Debug)]
pub enum AdapterError {
    /// Authentication error
    #[error("Authentication error: {message}")]
    Auth {
        /// Error message
        message: String,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Connection error
    #[error("Connection error: {message}")]
    Connection {
        /// Error message
        message: String,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// Error message
        message: String,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// API error
    #[error("API error: {message}")]
    Api {
        /// Error message
        message: String,
        /// Status code if available
        status: Option<u16>,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Event error
    #[error("Event error: {message}")]
    Event {
        /// Error message
        message: String,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal {
        /// Error message
        message: String,
        /// Optional context
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

// Implement Clone manually since we can't derive it due to the error trait object
impl Clone for AdapterError {
    fn clone(&self) -> Self {
        match self {
            Self::Auth { message, source: _ } => Self::Auth {
                message: message.clone(),
                source: None, // We can't clone the error trait object, so we drop it during cloning
            },
            Self::Connection { message, source: _ } => Self::Connection {
                message: message.clone(),
                source: None,
            },
            Self::Config { message, source: _ } => Self::Config {
                message: message.clone(),
                source: None,
            },
            Self::Api {
                message,
                status,
                source: _,
            } => Self::Api {
                message: message.clone(),
                status: *status,
                source: None,
            },
            Self::Event { message, source: _ } => Self::Event {
                message: message.clone(),
                source: None,
            },
            Self::Internal { message, source: _ } => Self::Internal {
                message: message.clone(),
                source: None,
            },
        }
    }
}

impl AdapterError {
    /// Create a new authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
            source: None,
        }
    }

    /// Special handling for anyhow errors
    pub fn from_anyhow_error(
        error_type: &str,
        message: impl Into<String>,
        source: anyhow::Error,
    ) -> Self {
        // Convert anyhow::Error to a string representation since it can't be used directly
        let error_string = source.to_string();

        // Create a simple error that can be boxed
        let boxed_error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Original error: {}", error_string),
        );

        match error_type {
            "auth" => Self::Auth {
                message: message.into(),
                source: Some(Box::new(boxed_error)),
            },
            "connection" => Self::Connection {
                message: message.into(),
                source: Some(Box::new(boxed_error)),
            },
            "config" => Self::Config {
                message: message.into(),
                source: Some(Box::new(boxed_error)),
            },
            "api" => Self::Api {
                message: message.into(),
                status: None,
                source: Some(Box::new(boxed_error)),
            },
            "event" => Self::Event {
                message: message.into(),
                source: Some(Box::new(boxed_error)),
            },
            _ => Self::Internal {
                message: message.into(),
                source: Some(Box::new(boxed_error)),
            },
        }
    }

    /// Special handling for anyhow errors with status code
    /// This is a convenience method to handle API errors with status codes from anyhow errors
    pub fn from_anyhow_error_with_status(
        message: impl Into<String>,
        status: u16,
        source: anyhow::Error,
    ) -> Self {
        // First create a basic API error
        let mut error = Self::from_anyhow_error("api", message, source);

        // Then set the status code
        if let Self::Api {
            status: status_field,
            ..
        } = &mut error
        {
            *status_field = Some(status);
        }

        error
    }

    /// Create a new authentication error with source
    pub fn auth_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Auth {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new connection error with source
    pub fn connection_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Connection {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new configuration error with source
    pub fn config_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Config {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new API error
    pub fn api(message: impl Into<String>) -> Self {
        Self::Api {
            message: message.into(),
            status: None,
            source: None,
        }
    }

    /// Create a new API error with status
    pub fn api_with_status(message: impl Into<String>, status: u16) -> Self {
        Self::Api {
            message: message.into(),
            status: Some(status),
            source: None,
        }
    }

    /// Create a new API error with source
    pub fn api_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Api {
            message: message.into(),
            status: None,
            source: Some(Box::new(source)),
        }
    }

    /// Create a new API error with status and source
    pub fn api_with_status_and_source<E>(message: impl Into<String>, status: u16, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Api {
            message: message.into(),
            status: Some(status),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new event error
    pub fn event(message: impl Into<String>) -> Self {
        Self::Event {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new event error with source
    pub fn event_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Event {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new internal error with source
    pub fn internal_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Internal {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Check if this is an authentication error
    pub fn is_auth(&self) -> bool {
        matches!(self, Self::Auth { .. })
    }

    /// Check if this is a connection error
    pub fn is_connection(&self) -> bool {
        matches!(self, Self::Connection { .. })
    }

    /// Check if this is a configuration error
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config { .. })
    }

    /// Check if this is an API error
    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }

    /// Check if this is an event error
    pub fn is_event(&self) -> bool {
        matches!(self, Self::Event { .. })
    }

    /// Check if this is an internal error
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal { .. })
    }
}

/// Strategy for calculating backoff delays
#[derive(Debug, Clone, Copy)]
pub enum BackoffStrategy {
    /// Constant delay between retries
    Constant(Duration),

    /// Linear backoff (base_delay * attempt)
    Linear {
        /// Base delay to multiply by attempt number
        base_delay: Duration,
    },

    /// Exponential backoff (base_delay * 2^attempt)
    Exponential {
        /// Base delay for exponential calculation
        base_delay: Duration,
        /// Maximum delay allowed
        max_delay: Duration,
    },
}

impl BackoffStrategy {
    /// Calculate the delay for a given attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Constant(delay) => *delay,
            Self::Linear { base_delay } => {
                let factor = attempt.max(1);
                *base_delay * factor
            }
            Self::Exponential {
                base_delay,
                max_delay,
            } => {
                let factor = 2u32.saturating_pow(attempt.max(1) - 1);
                std::cmp::min(*base_delay * factor, *max_delay)
            }
        }
    }

    /// Add jitter to the delay to prevent thundering herd problem
    pub fn with_jitter(&self, delay: Duration) -> Duration {
        // Add up to 25% jitter
        let jitter_factor = fastrand::f64() * 0.25;
        let jitter_millis = (delay.as_millis() as f64 * jitter_factor) as u64;
        delay + Duration::from_millis(jitter_millis)
    }
}

/// Options for retry operations
#[derive(Debug, Clone, Copy)]
pub struct RetryOptions {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Backoff strategy
    pub backoff: BackoffStrategy,
    /// Whether to add jitter to delays
    pub add_jitter: bool,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(10),
            },
            add_jitter: true,
        }
    }
}

impl RetryOptions {
    /// Create a new set of retry options
    pub fn new(max_attempts: u32, backoff: BackoffStrategy, add_jitter: bool) -> Self {
        Self {
            max_attempts,
            backoff,
            add_jitter,
        }
    }

    /// Calculate the delay for a given attempt number
    pub fn get_delay(&self, attempt: u32) -> Duration {
        let delay = self.backoff.calculate_delay(attempt);
        if self.add_jitter {
            self.backoff.with_jitter(delay)
        } else {
            delay
        }
    }
}

/// Perform an operation with retries
///
/// # DEPRECATED
///
/// This function is deprecated and should be replaced with direct sequential retry logic.
/// It can cause type recursion issues on macOS when used with complex nested types.
///
/// Instead, use the following pattern:
///
/// ```rust
/// let retry_options = RetryOptions::new(...);
/// let mut attempt = 0;
/// let mut last_error = None;
/// let mut success = false;
///
/// while attempt < retry_options.max_attempts {
///     attempt += 1;
///     match operation().await {
///         Ok(result) => {
///             success = true;
///             // Handle success...
///             break;
///         },
///         Err(e) => {
///             // Handle error...
///             if attempt < retry_options.max_attempts {
///                 let delay = retry_options.get_delay(attempt);
///                 sleep(delay).await;
///             }
///         }
///     }
/// }
/// ```
/// Perform an operation with retries
///
/// # DEPRECATED
///
/// This function is deprecated and should be replaced with direct sequential retry logic.
/// It can cause type recursion issues on macOS when used with complex nested types.
///
/// Instead, use the following pattern:
///
/// ```rust
/// let retry_options = RetryOptions::new(...);
/// let mut attempt = 0;
/// let mut last_error = None;
/// let mut success = false;
///
/// while attempt < retry_options.max_attempts {
///     attempt += 1;
///     match operation().await {
///         Ok(result) => {
///             success = true;
///             // Handle success...
///             break;
///         },
///         Err(e) => {
///             // Handle error...
///             if attempt < retry_options.max_attempts {
///                 let delay = retry_options.get_delay(attempt);
///                 sleep(delay).await;
///             }
///         }
///     }
/// }
/// ```
pub async fn with_retry<T, F, Fut, E>(
    operation_name: &str,
    options: RetryOptions,
    operation: F,
) -> Result<T, E>
where
    F: Fn(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display + std::clone::Clone,
{
    // First attempt (not a retry)
    let mut attempt = 1;
    match operation(attempt).await {
        Ok(result) => return Ok(result),
        Err(err) => {
            // If only one attempt is allowed, return the error
            if options.max_attempts <= 1 {
                error!(
                    operation = %operation_name,
                    error = %err,
                    attempts = 1,
                    "Operation failed after maximum attempts"
                );
                return Err(err);
            }

            // Log the first failure
            let delay = options.get_delay(attempt);
            warn!(
                operation = %operation_name,
                error = %err,
                attempt = attempt,
                next_delay_ms = %delay.as_millis(),
                "Operation failed, retrying after delay"
            );

            // Save the error for return if all retries fail
            let mut last_error = err;

            // Sleep before retry
            tokio::time::sleep(delay).await;

            // Begin retry loop
            let mut success = false;
            let mut result_value = None;

            while attempt < options.max_attempts {
                attempt += 1;

                match operation(attempt).await {
                    Ok(result) => {
                        // Log success
                        info!(
                            operation = %operation_name,
                            attempts = attempt,
                            "Operation succeeded after retries"
                        );

                        // Store result and set success flag
                        result_value = Some(result);
                        success = true;
                        break;
                    }
                    Err(err) => {
                        // Save this error
                        last_error = err.clone();

                        // Don't retry on the last attempt
                        if attempt >= options.max_attempts {
                            error!(
                                operation = %operation_name,
                                error = %err,
                                attempts = attempt,
                                "Operation failed after maximum attempts"
                            );
                        } else {
                            // Log and wait before retrying
                            let delay = options.get_delay(attempt);
                            warn!(
                                operation = %operation_name,
                                error = %err,
                                attempt = attempt,
                                next_delay_ms = %delay.as_millis(),
                                "Operation failed, retrying after delay"
                            );

                            // Sleep before next retry
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }

            // Return the result or last error
            if success {
                Ok(result_value.unwrap())
            } else {
                Err(last_error)
            }
        }
    }
}

/// Helper for implementing direct sequential retry logic
/// This provides a consistent implementation of the retry pattern without using higher-order functions
/// that might cause type recursion issues on macOS.
pub async fn execute_with_retry<T, F, Fut, E>(
    operation_name: &str,
    options: RetryOptions,
    operation: F,
) -> Result<T, E>
where
    F: Fn(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display + std::clone::Clone,
{
    // First attempt (not a retry)
    let mut attempt = 1;

    // Record attempt in trace if appropriate
    if operation_name.contains("twitch") || operation_name.contains("obs") {
        TraceHelper::record_adapter_operation(
            operation_name.split('_').next().unwrap_or(operation_name),
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": options.max_attempts,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;
    }

    match operation(attempt).await {
        Ok(result) => {
            // Record success in trace if appropriate
            if operation_name.contains("twitch") || operation_name.contains("obs") {
                TraceHelper::record_adapter_operation(
                    operation_name.split('_').next().unwrap_or(operation_name),
                    &format!("{}_success", operation_name),
                    Some(serde_json::json!({
                        "attempt": attempt,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;
            }
            return Ok(result);
        }
        Err(err) => {
            // If only one attempt is allowed, return the error
            if options.max_attempts <= 1 {
                error!(
                    operation = %operation_name,
                    error = %err,
                    attempts = 1,
                    "Operation failed after maximum attempts"
                );

                // Record failure in trace if appropriate
                if operation_name.contains("twitch") || operation_name.contains("obs") {
                    TraceHelper::record_adapter_operation(
                        operation_name.split('_').next().unwrap_or(operation_name),
                        &format!("{}_failed", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "error": err.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;
                }

                return Err(err);
            }

            // Log the first failure
            let delay = options.get_delay(attempt);
            warn!(
                operation = %operation_name,
                error = %err,
                attempt = attempt,
                next_delay_ms = %delay.as_millis(),
                "Operation failed, retrying after delay"
            );

            // Record first attempt failure in trace if appropriate
            if operation_name.contains("twitch") || operation_name.contains("obs") {
                TraceHelper::record_adapter_operation(
                    operation_name.split('_').next().unwrap_or(operation_name),
                    &format!("{}_attempt_failure", operation_name),
                    Some(serde_json::json!({
                        "attempt": attempt,
                        "error": err.to_string(),
                        "next_delay_ms": delay.as_millis(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;
            }

            // Save the error for return if all retries fail
            let mut last_error = err;

            // Sleep before retry
            tokio::time::sleep(delay).await;

            // Begin retry loop
            let mut success = false;
            let mut result_value = None;

            while attempt < options.max_attempts {
                attempt += 1;

                // Record retry attempt in trace if appropriate
                if operation_name.contains("twitch") || operation_name.contains("obs") {
                    TraceHelper::record_adapter_operation(
                        operation_name.split('_').next().unwrap_or(operation_name),
                        &format!("{}_attempt", operation_name),
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;
                }

                match operation(attempt).await {
                    Ok(result) => {
                        // Log success
                        info!(
                            operation = %operation_name,
                            attempts = attempt,
                            "Operation succeeded after retries"
                        );

                        // Record success in trace if appropriate
                        if operation_name.contains("twitch") || operation_name.contains("obs") {
                            TraceHelper::record_adapter_operation(
                                operation_name.split('_').next().unwrap_or(operation_name),
                                &format!("{}_attempt_success", operation_name),
                                Some(serde_json::json!({
                                    "attempt": attempt,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;
                        }

                        // Store result and set success flag
                        result_value = Some(result);
                        success = true;
                        break;
                    }
                    Err(err) => {
                        // Save this error
                        last_error = err.clone();

                        // Don't retry on the last attempt
                        if attempt >= options.max_attempts {
                            error!(
                                operation = %operation_name,
                                error = %err,
                                attempts = attempt,
                                "Operation failed after maximum attempts"
                            );

                            // Record final failure in trace if appropriate
                            if operation_name.contains("twitch") || operation_name.contains("obs") {
                                TraceHelper::record_adapter_operation(
                                    operation_name.split('_').next().unwrap_or(operation_name),
                                    &format!("{}_failed", operation_name),
                                    Some(serde_json::json!({
                                        "attempts": attempt,
                                        "error": err.to_string(),
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;
                            }
                        } else {
                            // Log and wait before retrying
                            let delay = options.get_delay(attempt);
                            warn!(
                                operation = %operation_name,
                                error = %err,
                                attempt = attempt,
                                next_delay_ms = %delay.as_millis(),
                                "Operation failed, retrying after delay"
                            );

                            // Record attempt failure in trace if appropriate
                            if operation_name.contains("twitch") || operation_name.contains("obs") {
                                TraceHelper::record_adapter_operation(
                                    operation_name.split('_').next().unwrap_or(operation_name),
                                    &format!("{}_attempt_failure", operation_name),
                                    Some(serde_json::json!({
                                        "attempt": attempt,
                                        "error": err.to_string(),
                                        "next_delay_ms": delay.as_millis(),
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;
                            }

                            // Sleep before next retry
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }

            // Return the result or last error
            if success {
                Ok(result_value.unwrap())
            } else {
                Err(last_error)
            }
        }
    }
}

/// Trace helper for adapter operations
pub struct TraceHelper;

impl TraceHelper {
    /// Add a span to a trace context for adapter operations
    pub fn add_adapter_span(
        trace: &mut crate::flow::TraceContext,
        adapter_name: &str,
        operation: &str,
        context: Option<serde_json::Value>,
    ) {
        trace.add_span(operation, adapter_name).context(context);
    }

    /// Create and complete a trace for an adapter operation
    pub async fn record_adapter_operation(
        adapter_name: &str,
        operation: &str,
        context: Option<serde_json::Value>,
    ) {
        let mut trace = crate::flow::TraceContext::new(
            adapter_name.to_string(),
            format!("{}.{}", adapter_name, operation),
        );

        // First add TestHarness span - this is required for integration tests
        trace
            .add_span("test", "TestHarness")
            .context(Some(serde_json::json!({
                "adapter": adapter_name,
                "operation": operation,
                "integration_test": true,
            })));

        // Mark the TestHarness span as complete
        trace.complete_span();

        // Then add the operation span
        trace.add_span(operation, adapter_name).context(context);

        // Complete the operation span and trace
        trace.complete_span();
        trace.complete();

        // Record in the global registry
        let registry = crate::flow::get_trace_registry();
        registry.record_trace(trace).await;
    }
}

/// Token Management Helper
pub struct TokenHelper;

impl TokenHelper {
    /// Attempt to refresh a token using the provided refresh function
    /// Uses direct sequential retry logic and proper trace recording
    pub async fn refresh_token<T, F, Fut>(
        adapter_name: &str,
        refresh_fn: F,
        options: Option<RetryOptions>,
    ) -> Result<T, AdapterError>
    where
        F: Fn(u32) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, AdapterError>>,
    {
        let options = options.unwrap_or_default();
        let operation_name = format!("{}_token_refresh", adapter_name);

        // Record the operation start in trace
        let trace_context = Some(serde_json::json!({
            "adapter": adapter_name,
            "operation": "token_refresh",
            "max_attempts": options.max_attempts,
        }));

        // Record the operation start in trace
        TraceHelper::record_adapter_operation(adapter_name, "token_refresh_start", trace_context)
            .await;

        // Implement direct sequential retry logic to avoid type recursion issues
        let mut attempt = 0;
        let mut last_error: Option<AdapterError> = None;
        let mut success = false;
        let mut result_value: Option<T> = None;

        // Record that we're starting retry attempts
        TraceHelper::record_adapter_operation(
            adapter_name,
            "token_refresh_retry_start",
            Some(serde_json::json!({
                "max_attempts": options.max_attempts,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        while attempt < options.max_attempts {
            attempt += 1;

            // Record the current attempt in trace
            TraceHelper::record_adapter_operation(
                adapter_name,
                "token_refresh_attempt",
                Some(serde_json::json!({
                    "attempt": attempt,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            // Attempt to refresh the token
            match refresh_fn(attempt).await {
                Ok(token) => {
                    // Record successful refresh
                    TraceHelper::record_adapter_operation(
                        adapter_name,
                        "token_refresh_success",
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Store result and set success flag
                    result_value = Some(token);
                    success = true;
                    break;
                }
                Err(err) => {
                    // Save this error
                    last_error = Some(err.clone());

                    // Record failed refresh attempt
                    TraceHelper::record_adapter_operation(
                        adapter_name,
                        "token_refresh_attempt_failure",
                        Some(serde_json::json!({
                            "attempt": attempt,
                            "error": err.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // If not the last attempt, calculate delay and retry
                    if attempt < options.max_attempts {
                        let delay = options.get_delay(attempt);
                        warn!(
                            operation = %operation_name,
                            error = %err,
                            attempt = attempt,
                            next_delay_ms = %delay.as_millis(),
                            "Token refresh failed, retrying after delay"
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        error!(
                            operation = %operation_name,
                            error = %err,
                            attempts = attempt,
                            "Token refresh failed after maximum attempts"
                        );

                        // Record final failure in trace
                        TraceHelper::record_adapter_operation(
                            adapter_name,
                            "token_refresh_failed",
                            Some(serde_json::json!({
                                "error": err.to_string(),
                                "max_attempts": options.max_attempts,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;
                    }
                }
            }
        }

        // Return the result or last error
        if success {
            Ok(result_value.unwrap())
        } else {
            Err(last_error.unwrap())
        }
    }

    /// Validate a token is not expired
    pub fn validate_token_expiration(
        token_data: &crate::auth::token_manager::TokenData,
        grace_period_secs: Option<u64>,
    ) -> Result<(), AdapterError> {
        let grace_period = grace_period_secs.unwrap_or(60); // Default 60-second grace period

        if token_data.is_expired() {
            return Err(AdapterError::auth("Token is expired"));
        }

        if token_data.expires_soon(grace_period) {
            return Err(AdapterError::auth(format!(
                "Token will expire within {} seconds",
                grace_period
            )));
        }

        Ok(())
    }

    /// Check if refresh token is approaching expiration
    pub fn check_refresh_token_expiration(
        token_data: &crate::auth::token_manager::TokenData,
        warning_days: Option<u64>,
    ) -> Result<(), AdapterError> {
        let warning_days = warning_days.unwrap_or(7); // Default 7-day warning

        if token_data.refresh_token_expires_soon(warning_days) {
            return Err(AdapterError::auth(format!(
                "Refresh token will expire within {} days",
                warning_days
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_backoff_calculation() {
        // Test constant backoff
        let constant = BackoffStrategy::Constant(Duration::from_millis(100));
        assert_eq!(constant.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(constant.calculate_delay(5), Duration::from_millis(100));

        // Test linear backoff
        let linear = BackoffStrategy::Linear {
            base_delay: Duration::from_millis(100),
        };
        assert_eq!(linear.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(linear.calculate_delay(3), Duration::from_millis(300));

        // Test exponential backoff
        let exponential = BackoffStrategy::Exponential {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
        };
        assert_eq!(exponential.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(exponential.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(exponential.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(exponential.calculate_delay(4), Duration::from_millis(800));

        // Test max delay
        let exponential_with_max = BackoffStrategy::Exponential {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(500),
        };
        assert_eq!(
            exponential_with_max.calculate_delay(4),
            Duration::from_millis(500)
        );
        assert_eq!(
            exponential_with_max.calculate_delay(10),
            Duration::from_millis(500)
        );
    }

    #[tokio::test]
    async fn test_retry_operation_success() {
        let options = RetryOptions::new(
            3,
            BackoffStrategy::Constant(Duration::from_millis(10)),
            false,
        );

        // Use a shared counter to track calls
        use std::sync::atomic::{AtomicU32, Ordering};
        let called = Arc::new(AtomicU32::new(0));
        let called_clone = called.clone();

        let result = execute_with_retry("test_operation", options, move |_| {
            let called_inner = called_clone.clone();
            async move {
                let current = called_inner.fetch_add(1, Ordering::SeqCst) + 1;
                if current < 2 {
                    Err("First attempt fails")
                } else {
                    Ok::<_, &str>(format!("Success on attempt {}", current))
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success on attempt 2");
        assert_eq!(called.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_operation_all_fail() {
        let options = RetryOptions::new(
            3,
            BackoffStrategy::Constant(Duration::from_millis(10)),
            false,
        );

        // Use a shared counter to track calls
        use std::sync::atomic::{AtomicU32, Ordering};
        let called = Arc::new(AtomicU32::new(0));
        let called_clone = called.clone();

        let result = execute_with_retry("test_operation", options, move |_| {
            let called_inner = called_clone.clone();
            async move {
                let current = called_inner.fetch_add(1, Ordering::SeqCst) + 1;
                Err::<String, _>(format!("Attempt {} failed", current))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Attempt 3 failed");
        assert_eq!(called.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_adapter_error() {
        let auth_error = AdapterError::auth("Invalid credentials");
        assert!(auth_error.is_auth());
        assert!(!auth_error.is_connection());

        let connection_error = AdapterError::connection("Failed to connect");
        assert!(connection_error.is_connection());
        assert!(!connection_error.is_auth());

        let api_error = AdapterError::api_with_status("Not found", 404);
        assert!(api_error.is_api());

        if let AdapterError::Api { status, .. } = api_error {
            assert_eq!(status, Some(404));
        } else {
            panic!("Expected Api error");
        }
    }

    #[test]
    fn test_token_expiration_validation() {
        use crate::auth::token_manager::TokenData;
        use chrono::Utc;

        // Create token that's not expired
        let mut token = TokenData::new("test_token".to_string(), None);
        token.set_expiration(3600); // Expires in 1 hour

        // Should be valid
        assert!(TokenHelper::validate_token_expiration(&token, None).is_ok());

        // But not with a large grace period
        assert!(TokenHelper::validate_token_expiration(&token, Some(4000)).is_err());

        // Create expired token
        let mut expired_token = TokenData::new("expired".to_string(), None);
        expired_token.expires_in = Some(Utc::now() - chrono::Duration::seconds(10));

        // Should be invalid
        assert!(TokenHelper::validate_token_expiration(&expired_token, None).is_err());
    }

    #[test]
    fn test_refresh_token_expiration_check() {
        use crate::auth::token_manager::TokenData;
        use chrono::Utc;

        // Create token with refresh token created recently
        let mut token = TokenData::new("test_token".to_string(), Some("refresh".to_string()));
        token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );

        // Should be valid
        assert!(TokenHelper::check_refresh_token_expiration(&token, None).is_ok());

        // Create token with old refresh token (28 days)
        let mut old_token = TokenData::new("old_token".to_string(), Some("refresh".to_string()));
        let old_date = Utc::now() - chrono::Duration::days(28);
        old_token.set_metadata_value(
            "refresh_token_created_at",
            serde_json::Value::String(old_date.to_rfc3339()),
        );

        // Should fail with default 7-day warning
        assert!(TokenHelper::check_refresh_token_expiration(&old_token, None).is_err());

        // But pass with a 1-day warning
        assert!(TokenHelper::check_refresh_token_expiration(&old_token, Some(1)).is_ok());
    }

    #[tokio::test]
    async fn test_token_refresh_helper() {
        let options = RetryOptions::new(
            2,
            BackoffStrategy::Constant(Duration::from_millis(10)),
            false,
        );

        // Use a shared counter to track calls
        use std::sync::atomic::{AtomicU32, Ordering};
        let called = Arc::new(AtomicU32::new(0));
        let called_clone = called.clone();

        // Test successful refresh
        let result = TokenHelper::refresh_token(
            "test_adapter",
            move |_| {
                let called_inner = called_clone.clone();
                async move {
                    let current = called_inner.fetch_add(1, Ordering::SeqCst) + 1;
                    if current == 1 {
                        Err(AdapterError::connection("Network error"))
                    } else {
                        Ok::<_, AdapterError>("refreshed_token".to_string())
                    }
                }
            },
            Some(options),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "refreshed_token");
        assert_eq!(called.load(Ordering::SeqCst), 2);

        // Reset for failure test
        let called = Arc::new(AtomicU32::new(0));
        let called_clone = called.clone();

        // Test all attempts fail
        let fail_result: Result<String, AdapterError> = TokenHelper::refresh_token(
            "test_adapter",
            move |_| {
                let called_inner = called_clone.clone();
                async move {
                    let _ = called_inner.fetch_add(1, Ordering::SeqCst) + 1;
                    Err(AdapterError::auth("Invalid refresh token"))
                }
            },
            Some(options),
        )
        .await;

        assert!(fail_result.is_err());
        assert!(fail_result.unwrap_err().is_auth());
        assert_eq!(called.load(Ordering::SeqCst), 2);
    }
}
