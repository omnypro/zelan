use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use tauri::async_runtime::RwLock;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Structured error type for Zelan application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZelanError {
    /// Error code for programmatic handling
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional context for additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Severity level
    pub severity: ErrorSeverity,
    /// Error category for retry policies and handling strategies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ErrorCategory>,
    /// Unique identifier for this error instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_id: Option<String>,
}

impl ZelanError {
    /// Create a new error builder with the specified error code
    pub fn new(code: ErrorCode) -> ZelanErrorBuilder {
        ZelanErrorBuilder {
            code,
            message: String::new(),
            context: None,
            severity: ErrorSeverity::Error,
            category: None,
            error_id: None,
        }
    }
}

/// Builder for creating ZelanError instances
pub struct ZelanErrorBuilder {
    code: ErrorCode,
    message: String,
    context: Option<String>,
    severity: ErrorSeverity,
    category: Option<ErrorCategory>,
    error_id: Option<String>,
}

impl ZelanErrorBuilder {
    /// Set the error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
    
    /// Set the error context
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
    
    /// Set the error severity
    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }
    
    /// Set the error category
    pub fn category(mut self, category: ErrorCategory) -> Self {
        self.category = Some(category);
        self
    }
    
    /// Set the error ID
    pub fn error_id(mut self, id: impl Into<String>) -> Self {
        self.error_id = Some(id.into());
        self
    }
    
    /// Build the final ZelanError
    pub fn build(self) -> ZelanError {
        ZelanError {
            code: self.code,
            message: self.message,
            context: self.context,
            severity: self.severity,
            category: self.category,
            error_id: self.error_id,
        }
    }
}

/// Error codes for different types of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // General errors
    Unknown,
    Internal,

    // Adapter related errors
    AdapterNotFound,
    AdapterConnectionFailed,
    AdapterDisconnectFailed,
    AdapterDisabled,

    // Event bus related errors
    EventBusPublishFailed,
    EventBusDropped,

    // HTTP/WebSocket related errors
    WebSocketBindFailed,
    WebSocketAcceptFailed,
    WebSocketSendFailed,

    // API related errors
    ApiRequestFailed,
    ApiRateLimited,
    ApiAuthenticationFailed,
    ApiPermissionDenied,

    // Authentication errors
    AuthTokenExpired,
    AuthTokenInvalid,
    AuthTokenRevoked,
    AuthRefreshFailed,

    // Network errors
    NetworkTimeout,
    NetworkConnectionLost,
    NetworkDnsFailure,

    // Configuration related errors
    ConfigInvalid,
    ConfigMissing,
}

/// Zelan Error types using thiserror
#[derive(Error, Debug, Clone)]
pub enum ZelanErrorType {
    // General errors
    #[error("Unknown error: {0}")]
    Unknown(String),
    
    #[error("Internal error: {0}")]
    Internal(String),

    // Adapter related errors
    #[error("Adapter '{name}' not found")]
    AdapterNotFound { 
        name: String,
    },
    
    #[error("Failed to connect adapter '{name}': {reason}")]
    AdapterConnectionFailed { 
        name: String,
        reason: String,
    },
    
    #[error("Failed to disconnect adapter '{name}': {reason}")]
    AdapterDisconnectFailed {
        name: String,
        reason: String,
    },
    
    #[error("Adapter '{name}' is disabled")]
    AdapterDisabled {
        name: String,
    },

    // Event bus related errors
    #[error("Failed to publish event to event bus: {reason}")]
    EventBusPublishFailed {
        reason: String,
    },
    
    #[error("Event was dropped due to no receivers")]
    EventBusDropped,

    // HTTP/WebSocket related errors
    #[error("Failed to bind WebSocket server: {reason}")]
    WebSocketBindFailed {
        reason: String,
    },
    
    #[error("Failed to accept WebSocket connection: {reason}")]
    WebSocketAcceptFailed {
        reason: String,
    },
    
    #[error("Failed to send WebSocket message: {reason}")]
    WebSocketSendFailed {
        reason: String,
    },

    // API related errors
    #[error("API request failed: {reason}")]
    ApiRequestFailed {
        reason: String,
    },
    
    #[error("{service} API rate limit exceeded")]
    ApiRateLimited {
        service: String,
        reset_after: Option<Duration>,
    },
    
    #[error("API authentication failed: {reason}")]
    ApiAuthenticationFailed {
        reason: String,
    },
    
    #[error("API permission denied: {reason}")]
    ApiPermissionDenied {
        reason: String,
    },

    // Authentication errors
    #[error("Authentication token has expired")]
    AuthTokenExpired,
    
    #[error("Authentication token is invalid: {reason}")]
    AuthTokenInvalid {
        reason: String,
    },
    
    #[error("Authentication token has been revoked")]
    AuthTokenRevoked,
    
    #[error("Failed to refresh authentication token: {reason}")]
    AuthRefreshFailed {
        reason: String,
    },

    // Network errors
    #[error("Network timeout while connecting to {service} for operation {operation}")]
    NetworkTimeout {
        service: String,
        operation: String,
    },
    
    #[error("Network connection lost to {service}")]
    NetworkConnectionLost {
        service: String,
    },
    
    #[error("DNS resolution failure for {host}")]
    NetworkDnsFailure {
        host: String,
    },

    // Configuration related errors
    #[error("Invalid configuration value for '{key}': {reason}")]
    ConfigInvalid {
        key: String,
        value: String,
        reason: String,
    },
    
    #[error("Required configuration key '{key}' is missing")]
    ConfigMissing {
        key: String,
    },
}

/// Severity levels for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    /// Informational messages that don't impact functionality
    Info,
    /// Warnings that might impact functionality but don't stop operation
    Warning,
    /// Errors that impact functionality but allow continued operation
    Error,
    /// Critical errors that prevent the application from functioning properly
    Critical,
}

/// Error categories for different retry strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Temporary network issues, timeouts, etc. - usually retryable
    Network,
    /// Authentication/authorization failures that might be fixed by token refresh
    Authentication,
    /// API rate limiting - retryable with backoff
    RateLimit,
    /// API service unavailable - retryable with longer backoff
    ServiceUnavailable,
    /// Permission/access denied - not retryable without reconfiguration
    Permission,
    /// Configuration errors - not retryable without reconfiguration
    Configuration,
    /// Internal errors in our code - generally not retryable
    Internal,
    /// Resource not found - generally not retryable
    NotFound,
    /// Validation errors - not retryable without input changes
    Validation,
}

impl ErrorCategory {
    /// Returns true if errors in this category are generally retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network | Self::Authentication | Self::RateLimit | Self::ServiceUnavailable => {
                true
            }

            Self::Permission
            | Self::Configuration
            | Self::Internal
            | Self::NotFound
            | Self::Validation => false,
        }
    }

    /// Get the default retry policy for this error category
    pub fn default_retry_policy(&self) -> RetryPolicy {
        match self {
            Self::Network => RetryPolicy::exponential_backoff(
                5,
                Duration::from_millis(500),
                2.0,
                Some(Duration::from_secs(30)),
            ),
            Self::Authentication => RetryPolicy::fixed_delay(2, Duration::from_secs(2)),
            Self::RateLimit => RetryPolicy::exponential_backoff(
                3,
                Duration::from_secs(1),
                2.0,
                Some(Duration::from_secs(60)),
            ),
            Self::ServiceUnavailable => RetryPolicy::exponential_backoff(
                5,
                Duration::from_secs(5),
                2.0,
                Some(Duration::from_secs(120)),
            ),
            _ => RetryPolicy::no_retry(),
        }
    }
}

/// Retry policy for error recovery
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Backoff factor for exponential backoff
    pub backoff_factor: f64,
    /// Maximum delay between retries
    pub max_delay: Option<Duration>,
    /// Whether to add jitter to the delay
    pub use_jitter: bool,
}

impl RetryPolicy {
    /// Create a new retry policy with no retries
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            base_delay: Duration::from_millis(0),
            backoff_factor: 1.0,
            max_delay: None,
            use_jitter: false,
        }
    }

    /// Create a new retry policy with a fixed delay between retries
    pub fn fixed_delay(max_retries: usize, delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay: delay,
            backoff_factor: 1.0,
            max_delay: None,
            use_jitter: false,
        }
    }

    /// Create a new retry policy with exponential backoff
    pub fn exponential_backoff(
        max_retries: usize,
        base_delay: Duration,
        backoff_factor: f64,
        max_delay: Option<Duration>,
    ) -> Self {
        Self {
            max_retries,
            base_delay,
            backoff_factor,
            max_delay,
            use_jitter: true,
        }
    }

    /// Calculate the delay for a specific retry attempt
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 || attempt > self.max_retries {
            return Duration::from_millis(0);
        }

        // Calculate the base delay with exponential backoff
        let mut delay_ms =
            self.base_delay.as_millis() as f64 * self.backoff_factor.powf((attempt - 1) as f64);

        // Apply jitter if enabled (±25%)
        if self.use_jitter {
            // Use a simple deterministic jitter based on the attempt number
            // This avoids needing to import a random number generator
            let jitter_factor = 0.75 + ((attempt as f64 * 0.15) % 0.5);
            delay_ms *= jitter_factor;
        }

        // Cap at max_delay if specified
        if let Some(max_delay) = self.max_delay {
            delay_ms = delay_ms.min(max_delay.as_millis() as f64);
        }

        Duration::from_millis(delay_ms as u64)
    }
}

/// Error registry for tracking and analyzing errors
#[derive(Debug)]
pub struct ErrorRegistry {
    /// Map of error codes to counts and timestamps
    error_counts: RwLock<HashMap<ErrorCode, usize>>,
    /// Map of error codes to last occurrence
    last_occurrence: RwLock<HashMap<ErrorCode, Instant>>,
    /// Map of error sources to counts
    source_counts: RwLock<HashMap<String, usize>>,
    /// Queue of errors in FIFO order
    error_history: RwLock<VecDeque<ZelanError>>,
    /// Map of error IDs to indices in the error_history queue for fast lookups
    error_id_map: RwLock<HashMap<String, usize>>,
    /// Maximum number of errors to store in history
    max_history: usize,
}

impl ErrorRegistry {
    /// Create a new error registry
    pub fn new(max_history: usize) -> Self {
        Self {
            error_counts: RwLock::new(HashMap::new()),
            last_occurrence: RwLock::new(HashMap::new()),
            source_counts: RwLock::new(HashMap::new()),
            error_history: RwLock::new(VecDeque::with_capacity(max_history)),
            error_id_map: RwLock::new(HashMap::new()),
            max_history,
        }
    }

    /// Register an error with the registry
    pub async fn register(&self, mut error: ZelanError, source: Option<&str>) -> ZelanError {
        // Generate an error ID if not present
        if error.error_id.is_none() {
            let now = chrono::Utc::now();
            let id = format!("err-{}-{}", now.timestamp_millis(), fastrand::u32(..));
            error.error_id = Some(id);
        }

        // Update error counts
        {
            let mut counts = self.error_counts.write().await;
            *counts.entry(error.code).or_insert(0) += 1;
        }

        // Update last occurrence
        {
            let mut last = self.last_occurrence.write().await;
            last.insert(error.code, Instant::now());
        }

        // Update source counts if provided
        if let Some(src) = source {
            let mut sources = self.source_counts.write().await;
            *sources.entry(src.to_string()).or_insert(0) += 1;
        }

        // Add to history with proper FIFO eviction
        {
            // First ensure we have an error ID
            if let Some(id) = &error.error_id {
                let mut history = self.error_history.write().await;
                let mut id_map = self.error_id_map.write().await;
                
                // If we've reached capacity, remove the oldest error
                if history.len() >= self.max_history {
                    if let Some(old_error) = history.pop_front() {
                        // Also remove from the ID map
                        if let Some(old_id) = old_error.error_id {
                            id_map.remove(&old_id);
                        }
                        
                        // Update indices in the map after removal
                        for index in id_map.values_mut() {
                            *index -= 1;
                        }
                    }
                }
                
                // Add new error to the end of the queue
                history.push_back(error.clone());
                
                // Store its index in the ID map
                id_map.insert(id.clone(), history.len() - 1);
            }
        }

        error
    }

    /// Get count for a specific error code
    pub async fn get_count(&self, code: ErrorCode) -> usize {
        self.error_counts
            .read()
            .await
            .get(&code)
            .copied()
            .unwrap_or(0)
    }

    /// Get the last occurrence time for a specific error code
    pub async fn get_last_occurrence(&self, code: ErrorCode) -> Option<Instant> {
        self.last_occurrence.read().await.get(&code).copied()
    }

    /// Get all error statistics
    pub async fn get_stats(&self) -> ErrorStats {
        ErrorStats {
            counts: self.error_counts.read().await.clone(),
            source_counts: self.source_counts.read().await.clone(),
            error_count: self.error_history.read().await.len(),
        }
    }

    /// Get the full error history
    pub async fn get_history(&self) -> Vec<ZelanError> {
        self.error_history.read().await.iter().cloned().collect()
    }

    /// Get a specific error by ID
    pub async fn get_error_by_id(&self, id: &str) -> Option<ZelanError> {
        let id_map = self.error_id_map.read().await;
        let history = self.error_history.read().await;
        
        id_map.get(id).and_then(|&index| history.get(index).cloned())
    }

    /// Clear all error statistics
    pub async fn clear(&self) {
        *self.error_counts.write().await = HashMap::new();
        *self.last_occurrence.write().await = HashMap::new();
        *self.source_counts.write().await = HashMap::new();
        *self.error_history.write().await = VecDeque::new();
        *self.error_id_map.write().await = HashMap::new();
    }
}

/// Error statistics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    /// Map of error codes to counts
    pub counts: HashMap<ErrorCode, usize>,
    /// Map of error sources to counts
    pub source_counts: HashMap<String, usize>,
    /// Total number of errors in history
    pub error_count: usize,
}

impl fmt::Display for ZelanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "{}: {} ({})", self.code, self.message, context)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::Unknown => write!(f, "UNKNOWN"),
            ErrorCode::Internal => write!(f, "INTERNAL"),
            ErrorCode::AdapterNotFound => write!(f, "ADAPTER_NOT_FOUND"),
            ErrorCode::AdapterConnectionFailed => write!(f, "ADAPTER_CONNECTION_FAILED"),
            ErrorCode::AdapterDisconnectFailed => write!(f, "ADAPTER_DISCONNECT_FAILED"),
            ErrorCode::AdapterDisabled => write!(f, "ADAPTER_DISABLED"),
            ErrorCode::EventBusPublishFailed => write!(f, "EVENT_BUS_PUBLISH_FAILED"),
            ErrorCode::EventBusDropped => write!(f, "EVENT_BUS_DROPPED"),
            ErrorCode::WebSocketBindFailed => write!(f, "WEBSOCKET_BIND_FAILED"),
            ErrorCode::WebSocketAcceptFailed => write!(f, "WEBSOCKET_ACCEPT_FAILED"),
            ErrorCode::WebSocketSendFailed => write!(f, "WEBSOCKET_SEND_FAILED"),
            ErrorCode::ApiRequestFailed => write!(f, "API_REQUEST_FAILED"),
            ErrorCode::ApiRateLimited => write!(f, "API_RATE_LIMITED"),
            ErrorCode::ApiAuthenticationFailed => write!(f, "API_AUTHENTICATION_FAILED"),
            ErrorCode::ApiPermissionDenied => write!(f, "API_PERMISSION_DENIED"),
            ErrorCode::AuthTokenExpired => write!(f, "AUTH_TOKEN_EXPIRED"),
            ErrorCode::AuthTokenInvalid => write!(f, "AUTH_TOKEN_INVALID"),
            ErrorCode::AuthTokenRevoked => write!(f, "AUTH_TOKEN_REVOKED"),
            ErrorCode::AuthRefreshFailed => write!(f, "AUTH_REFRESH_FAILED"),
            ErrorCode::NetworkTimeout => write!(f, "NETWORK_TIMEOUT"),
            ErrorCode::NetworkConnectionLost => write!(f, "NETWORK_CONNECTION_LOST"),
            ErrorCode::NetworkDnsFailure => write!(f, "NETWORK_DNS_FAILURE"),
            ErrorCode::ConfigInvalid => write!(f, "CONFIG_INVALID"),
            ErrorCode::ConfigMissing => write!(f, "CONFIG_MISSING"),
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::Authentication => write!(f, "Authentication"),
            ErrorCategory::RateLimit => write!(f, "RateLimit"),
            ErrorCategory::ServiceUnavailable => write!(f, "ServiceUnavailable"),
            ErrorCategory::Permission => write!(f, "Permission"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
            ErrorCategory::Internal => write!(f, "Internal"),
            ErrorCategory::NotFound => write!(f, "NotFound"),
            ErrorCategory::Validation => write!(f, "Validation"),
        }
    }
}

impl std::error::Error for ZelanError {}

impl From<ZelanErrorType> for ZelanError {
    fn from(err: ZelanErrorType) -> Self {
        let (code, message, _context, category) = match &err {
            ZelanErrorType::Unknown(msg) => 
                (ErrorCode::Unknown, msg.clone(), None, ErrorCategory::Internal),
                
            ZelanErrorType::Internal(msg) => 
                (ErrorCode::Internal, msg.clone(), None, ErrorCategory::Internal),
                
            ZelanErrorType::AdapterNotFound { name } => 
                (ErrorCode::AdapterNotFound, 
                 format!("Adapter '{}' not found", name), 
                 None, 
                 ErrorCategory::NotFound),
                 
            ZelanErrorType::AdapterConnectionFailed { name, reason } => 
                (ErrorCode::AdapterConnectionFailed, 
                 format!("Failed to connect adapter '{}'", name), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::AdapterDisconnectFailed { name, reason } => 
                (ErrorCode::AdapterDisconnectFailed, 
                 format!("Failed to disconnect adapter '{}'", name), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::AdapterDisabled { name } => 
                (ErrorCode::AdapterDisabled, 
                 format!("Adapter '{}' is disabled", name), 
                 None, 
                 ErrorCategory::Configuration),
                 
            ZelanErrorType::EventBusPublishFailed { reason } => 
                (ErrorCode::EventBusPublishFailed, 
                 "Failed to publish event to event bus".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Internal),
                 
            ZelanErrorType::EventBusDropped => 
                (ErrorCode::EventBusDropped, 
                 "Event was dropped due to no receivers".to_string(), 
                 None, 
                 ErrorCategory::Internal),
                 
            ZelanErrorType::WebSocketBindFailed { reason } => 
                (ErrorCode::WebSocketBindFailed, 
                 "Failed to bind WebSocket server".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::WebSocketAcceptFailed { reason } => 
                (ErrorCode::WebSocketAcceptFailed, 
                 "Failed to accept WebSocket connection".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::WebSocketSendFailed { reason } => 
                (ErrorCode::WebSocketSendFailed, 
                 "Failed to send WebSocket message".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::ApiRequestFailed { reason } => 
                (ErrorCode::ApiRequestFailed, 
                 "API request failed".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::ApiRateLimited { service, reset_after } => {
                let ctx = reset_after.map(|d| format!("Rate limit resets in {} seconds", d.as_secs()));
                (ErrorCode::ApiRateLimited, 
                 format!("{} API rate limit exceeded", service), 
                 ctx, 
                 ErrorCategory::RateLimit)
            },
                 
            ZelanErrorType::ApiAuthenticationFailed { reason } => 
                (ErrorCode::ApiAuthenticationFailed, 
                 "API authentication failed".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Authentication),
                 
            ZelanErrorType::ApiPermissionDenied { reason } => 
                (ErrorCode::ApiPermissionDenied, 
                 "API permission denied".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Permission),
                 
            ZelanErrorType::AuthTokenExpired => 
                (ErrorCode::AuthTokenExpired, 
                 "Authentication token has expired".to_string(), 
                 None, 
                 ErrorCategory::Authentication),
                 
            ZelanErrorType::AuthTokenInvalid { reason } => 
                (ErrorCode::AuthTokenInvalid, 
                 "Authentication token is invalid".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Authentication),
                 
            ZelanErrorType::AuthTokenRevoked => 
                (ErrorCode::AuthTokenRevoked, 
                 "Authentication token has been revoked".to_string(), 
                 None, 
                 ErrorCategory::Authentication),
                 
            ZelanErrorType::AuthRefreshFailed { reason } => 
                (ErrorCode::AuthRefreshFailed, 
                 "Failed to refresh authentication token".to_string(), 
                 Some(reason.clone()), 
                 ErrorCategory::Authentication),
                 
            ZelanErrorType::NetworkTimeout { service, operation } => 
                (ErrorCode::NetworkTimeout, 
                 format!("Network timeout while connecting to {}", service), 
                 Some(format!("Operation: {}", operation)), 
                 ErrorCategory::Network),
                 
            ZelanErrorType::NetworkConnectionLost { service } => 
                (ErrorCode::NetworkConnectionLost, 
                 format!("Network connection lost to {}", service), 
                 None, 
                 ErrorCategory::Network),
                 
            ZelanErrorType::NetworkDnsFailure { host } => 
                (ErrorCode::NetworkDnsFailure, 
                 format!("DNS resolution failure for {}", host), 
                 None, 
                 ErrorCategory::Network),
                 
            ZelanErrorType::ConfigInvalid { key, value, reason } => 
                (ErrorCode::ConfigInvalid, 
                 format!("Invalid configuration value for '{}'", key), 
                 Some(format!("Value '{}' is invalid: {}", value, reason)), 
                 ErrorCategory::Configuration),
                 
            ZelanErrorType::ConfigMissing { key } => 
                (ErrorCode::ConfigMissing, 
                 format!("Required configuration key '{}' is missing", key), 
                 None, 
                 ErrorCategory::Configuration),
        };

        ZelanError::new(code)
            .message(message)
            .category(category)
            .severity(ErrorSeverity::Error) // Default severity
            .build()
    }
}

impl From<anyhow::Error> for ZelanError {
    fn from(err: anyhow::Error) -> Self {
        // Try to extract error category from the error message
        let err_str = err.to_string().to_lowercase();
        let category = if !err_str.is_empty() {
            if err_str.contains("timeout") || err_str.contains("timed out") {
                ErrorCategory::Network
            } else if err_str.contains("unauthorized") || err_str.contains("token") {
                ErrorCategory::Authentication
            } else if err_str.contains("rate")
                && (err_str.contains("limit") || err_str.contains("exceeded"))
            {
                ErrorCategory::RateLimit
            } else if err_str.contains("permission") || err_str.contains("forbidden") {
                ErrorCategory::Permission
            } else if err_str.contains("not found") || err_str.contains("404") {
                ErrorCategory::NotFound
            } else if err_str.contains("invalid") || err_str.contains("validation") {
                ErrorCategory::Validation
            } else if err_str.contains("unavailable") || err_str.contains("503") {
                ErrorCategory::ServiceUnavailable
            } else {
                ErrorCategory::Internal
            }
        } else {
            ErrorCategory::Internal
        };

        ZelanError::new(ErrorCode::Internal)
            .message(err.to_string())
            .category(category)
            .severity(ErrorSeverity::Error)
            .build()
    }
}

// Custom Result type for Zelan operations
pub type ZelanResult<T> = Result<T, ZelanError>;

// New result type for thiserror-based operations
pub type ZelanTypeResult<T> = Result<T, ZelanErrorType>;

/// Module for new thiserror-based error helpers
pub mod errors {
    use super::*;
    
    /// Create an adapter not found error
    pub fn adapter_not_found(name: impl Into<String>) -> ZelanErrorType {
        ZelanErrorType::AdapterNotFound {
            name: name.into(),
        }
    }

    /// Create an adapter connection failed error
    pub fn adapter_connection_failed(name: impl Into<String>, reason: impl fmt::Display) -> ZelanErrorType {
        ZelanErrorType::AdapterConnectionFailed {
            name: name.into(),
            reason: reason.to_string(),
        }
    }

    /// Create an event bus publish failed error
    pub fn event_bus_publish_failed(reason: impl fmt::Display) -> ZelanErrorType {
        ZelanErrorType::EventBusPublishFailed {
            reason: reason.to_string(),
        }
    }

    /// Create an event bus dropped error
    pub fn event_bus_dropped() -> ZelanErrorType {
        ZelanErrorType::EventBusDropped
    }

    /// Create a WebSocket bind failed error
    pub fn websocket_bind_failed(reason: impl fmt::Display) -> ZelanErrorType {
        ZelanErrorType::WebSocketBindFailed {
            reason: reason.to_string(),
        }
    }

    /// Create a network timeout error
    pub fn network_timeout(service: impl Into<String>, operation: impl Into<String>) -> ZelanErrorType {
        ZelanErrorType::NetworkTimeout {
            service: service.into(),
            operation: operation.into(),
        }
    }

    /// Create an API rate limit error
    pub fn api_rate_limited(service: impl Into<String>, reset_after: Option<Duration>) -> ZelanErrorType {
        ZelanErrorType::ApiRateLimited {
            service: service.into(),
            reset_after,
        }
    }

    /// Create an authentication token expired error
    pub fn auth_token_expired() -> ZelanErrorType {
        ZelanErrorType::AuthTokenExpired
    }

    /// Create a configuration missing error
    pub fn config_missing(key: impl Into<String>) -> ZelanErrorType {
        ZelanErrorType::ConfigMissing {
            key: key.into(),
        }
    }

    /// Create a configuration invalid error
    pub fn config_invalid(
        key: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> ZelanErrorType {
        ZelanErrorType::ConfigInvalid {
            key: key.into(),
            value: value.into(),
            reason: reason.into(),
        }
    }
}

// Utility functions to create errors

/// Create an adapter not found error
pub fn adapter_not_found(name: &str) -> ZelanError {
    ZelanError::new(ErrorCode::AdapterNotFound)
        .message(format!("Adapter '{}' not found", name))
        .category(ErrorCategory::NotFound)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create an adapter connection failed error
pub fn adapter_connection_failed(name: &str, err: impl fmt::Display) -> ZelanError {
    ZelanError::new(ErrorCode::AdapterConnectionFailed)
        .message(format!("Failed to connect adapter '{}'", name))
        .context(err.to_string())
        .category(ErrorCategory::Network)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create an event bus publish failed error
pub fn event_bus_publish_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError::new(ErrorCode::EventBusPublishFailed)
        .message("Failed to publish event to event bus")
        .context(err.to_string())
        .category(ErrorCategory::Internal)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create an event bus dropped error
pub fn event_bus_dropped() -> ZelanError {
    ZelanError::new(ErrorCode::EventBusDropped)
        .message("Event was dropped due to no receivers")
        .category(ErrorCategory::Internal)
        .severity(ErrorSeverity::Warning)
        .build()
}

/// Create a WebSocket bind failed error
pub fn websocket_bind_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError::new(ErrorCode::WebSocketBindFailed)
        .message("Failed to bind WebSocket server")
        .context(err.to_string())
        .category(ErrorCategory::Network)
        .severity(ErrorSeverity::Critical)
        .build()
}

/// Create a WebSocket accept failed error
pub fn websocket_accept_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError::new(ErrorCode::WebSocketAcceptFailed)
        .message("Failed to accept WebSocket connection")
        .context(err.to_string())
        .category(ErrorCategory::Network)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create a WebSocket send failed error
pub fn websocket_send_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError::new(ErrorCode::WebSocketSendFailed)
        .message("Failed to send WebSocket message")
        .context(err.to_string())
        .category(ErrorCategory::Network)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create an API rate limit error
pub fn api_rate_limited(service: &str, reset_after: Option<Duration>) -> ZelanError {
    let mut builder = ZelanError::new(ErrorCode::ApiRateLimited)
        .message(format!("{} API rate limit exceeded", service))
        .category(ErrorCategory::RateLimit)
        .severity(ErrorSeverity::Warning);
    
    if let Some(duration) = reset_after {
        builder = builder.context(format!(
            "Rate limit resets in {} seconds",
            duration.as_secs()
        ));
    }
    
    builder.build()
}

/// Create an authentication error
pub fn auth_token_expired() -> ZelanError {
    ZelanError::new(ErrorCode::AuthTokenExpired)
        .message("Authentication token has expired")
        .category(ErrorCategory::Authentication)
        .severity(ErrorSeverity::Warning)
        .build()
}

/// Create a network timeout error
pub fn network_timeout(service: &str, operation: &str) -> ZelanError {
    ZelanError::new(ErrorCode::NetworkTimeout)
        .message(format!("Network timeout while connecting to {}", service))
        .context(format!("Operation: {}", operation))
        .category(ErrorCategory::Network)
        .severity(ErrorSeverity::Warning)
        .build()
}

/// Create a configuration error
pub fn config_invalid(key: &str, value: &str, reason: &str) -> ZelanError {
    ZelanError::new(ErrorCode::ConfigInvalid)
        .message(format!("Invalid configuration value for '{}'", key))
        .context(format!("Value '{}' is invalid: {}", value, reason))
        .category(ErrorCategory::Configuration)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Create a configuration missing error
pub fn config_missing(key: &str) -> ZelanError {
    ZelanError::new(ErrorCode::ConfigMissing)
        .message(format!("Required configuration key '{}' is missing", key))
        .category(ErrorCategory::Configuration)
        .severity(ErrorSeverity::Error)
        .build()
}

/// Helper for executing a function with automatic retries according to a retry policy
pub async fn with_retry<F, Fut, T>(
    operation_name: &str,
    retry_policy: &RetryPolicy,
    f: F,
) -> ZelanResult<T>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ZelanResult<T>> + Send,
{
    let mut attempt = 0;
    let max_retries = retry_policy.max_retries;

    loop {
        attempt += 1;
        match f().await {
            Ok(value) => {
                if attempt > 1 {
                    debug!(
                        operation = operation_name,
                        attempt, "Operation succeeded after {} attempts", attempt
                    );
                }
                return Ok(value);
            }
            Err(err) => {
                // Check if we should retry based on error category
                let should_retry = err.category.map(|cat| cat.is_retryable()).unwrap_or(false);

                if !should_retry || attempt > max_retries {
                    if attempt > 1 {
                        error!(
                            operation = operation_name,
                            attempt,
                            error = %err,
                            "Operation failed after {} attempts",
                            attempt
                        );
                    }
                    return Err(err);
                }

                // Calculate delay for this attempt
                let delay = retry_policy.calculate_delay(attempt);

                // Log the retry
                if err.severity == ErrorSeverity::Critical {
                    error!(
                        operation = operation_name,
                        attempt,
                        max_retries,
                        error = %err,
                        retry_after_ms = delay.as_millis(),
                        "Critical error, retrying operation"
                    );
                } else if err.severity == ErrorSeverity::Error {
                    warn!(
                        operation = operation_name,
                        attempt,
                        max_retries,
                        error = %err,
                        retry_after_ms = delay.as_millis(),
                        "Operation failed, retrying"
                    );
                } else {
                    debug!(
                        operation = operation_name,
                        attempt,
                        max_retries,
                        error = %err,
                        retry_after_ms = delay.as_millis(),
                        "Retrying operation"
                    );
                }

                // Wait before retrying
                sleep(delay).await;
            }
        }
    }
}

/// Circuit breaker for preventing repeated failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Name of the circuit breaker
    name: String,
    /// Maximum number of failures before opening the circuit
    failure_threshold: usize,
    /// Time to wait before attempting to close the circuit
    reset_timeout: Duration,
    /// Current failure count
    failure_count: AtomicUsize,
    /// Time when the circuit was opened
    open_time: RwLock<Option<Instant>>,
    /// Whether the circuit is currently open
    is_open: AtomicBool,
    /// Error registry for tracking errors
    error_registry: Option<Arc<ErrorRegistry>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: &str, failure_threshold: usize, reset_timeout: Duration) -> Self {
        Self {
            name: name.to_string(),
            failure_threshold,
            reset_timeout,
            failure_count: AtomicUsize::new(0),
            open_time: RwLock::new(None),
            is_open: AtomicBool::new(false),
            error_registry: None,
        }
    }

    /// Set the error registry
    pub fn with_error_registry(mut self, registry: Arc<ErrorRegistry>) -> Self {
        self.error_registry = Some(registry);
        self
    }

    /// Check if the circuit is open
    pub async fn is_open(&self) -> bool {
        let is_open = self.is_open.load(Ordering::Acquire);

        // If the circuit is open, check if we've waited long enough to try again
        if is_open {
            if let Some(open_time) = *self.open_time.read().await {
                if open_time.elapsed() >= self.reset_timeout {
                    // Reset to half-open state
                    self.is_open.store(false, Ordering::Release);
                    debug!(
                        name = self.name,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker reset attempt"
                    );
                    return false;
                }
            }
        }

        is_open
    }

    /// Record a success, resetting the failure count
    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Release);
        self.is_open.store(false, Ordering::Release);
        *self.open_time.write().await = None;
    }

    /// Record a failure, potentially opening the circuit
    pub async fn record_failure(&self, error: Option<&ZelanError>) {
        // Atomically increment the failure count
        let count = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;

        // Register the error if provided
        if let Some(err) = error {
            if let Some(registry) = &self.error_registry {
                let _ = registry.register(err.clone(), Some(&self.name)).await;
            }
        }

        // Open the circuit if we've reached the threshold
        if count >= self.failure_threshold {
            self.is_open.store(true, Ordering::Release);
            *self.open_time.write().await = Some(Instant::now());

            // Log at appropriate level based on error severity
            let severity = error.map(|e| e.severity).unwrap_or(ErrorSeverity::Error);
            match severity {
                ErrorSeverity::Critical => {
                    error!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened due to critical failures"
                    );
                }
                ErrorSeverity::Error => {
                    warn!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened due to repeated errors"
                    );
                }
                _ => {
                    info!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened"
                    );
                }
            }
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn execute<F, Fut, T>(&self, f: F) -> ZelanResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ZelanResult<T>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            let reset_in = self.reset_timeout - self.open_time.read().await.unwrap().elapsed();
            return Err(ZelanError::new(ErrorCode::Internal)
                .message(format!("Circuit breaker '{}' is open", self.name))
                .context(format!("Too many failures (reset in {:?})", reset_in))
                .severity(ErrorSeverity::Warning)
                .category(ErrorCategory::ServiceUnavailable)
                .build());
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(err) => {
                self.record_failure(Some(&err)).await;
                Err(err)
            }
        }
    }
}

/*
Example of using the new thiserror-based error API

```rust
// Using the original error API:
fn get_adapter(name: &str) -> ZelanResult<Adapter> {
    if name.is_empty() {
        return Err(adapter_not_found(name)); 
    }
    // ...
    Ok(adapter)
}

// Using the new thiserror-based API:
fn get_adapter_new(name: &str) -> ZelanTypeResult<Adapter> {
    if name.is_empty() {
        return Err(errors::adapter_not_found(name)); 
    }
    // ...
    Ok(adapter)
}

// Converting between error types:
fn get_adapter_compatible(name: &str) -> ZelanResult<Adapter> {
    match get_adapter_new(name) {
        Ok(adapter) => Ok(adapter),
        Err(err) => Err(err.into()),  // ZelanErrorType -> ZelanError
    }
}
```
*/

/// Simple circuit breaker for our custom implementation 
#[derive(Clone, Debug)]
pub struct SimpleCircuitBreaker {
    /// Name of the circuit breaker
    name: String,
    /// Maximum number of failures before opening the circuit
    failure_threshold: u32,
    /// Time to wait before attempting to close the circuit
    reset_timeout: Duration,
    /// Whether the circuit is currently open 
    is_open: Arc<AtomicBool>,
    /// Current failure count
    failure_count: Arc<AtomicUsize>,
    /// Time when the circuit was opened
    open_time: Arc<RwLock<Option<Instant>>>,
    /// Error registry for tracking errors
    error_registry: Option<Arc<ErrorRegistry>>,
}

impl SimpleCircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: &str, failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            name: name.to_string(),
            failure_threshold,
            reset_timeout,
            is_open: Arc::new(AtomicBool::new(false)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            open_time: Arc::new(RwLock::new(None)),
            error_registry: None,
        }
    }
    
    /// Set the error registry
    pub fn with_error_registry(mut self, registry: Arc<ErrorRegistry>) -> Self {
        self.error_registry = Some(registry);
        self
    }
    
    /// Check if the circuit is open, reset if timeout has passed
    pub async fn is_open(&self) -> bool {
        let is_open = self.is_open.load(Ordering::Acquire);
        
        // If the circuit is open, check if we've waited long enough to try again
        if is_open {
            let open_time = self.open_time.read().await;
            if let Some(opened_at) = *open_time {
                if opened_at.elapsed() >= self.reset_timeout {
                    // Reset to half-open state
                    self.is_open.store(false, Ordering::Release);
                    // We don't reset failure count immediately to implement half-open state
                    debug!(
                        name = self.name,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker reset attempt"
                    );
                    return false;
                }
            }
        }
        
        is_open
    }
    
    /// Record a success, resetting the failure count
    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Release);
        self.is_open.store(false, Ordering::Release);
        *self.open_time.write().await = None;
    }
    
    /// Record a failure, potentially opening the circuit
    pub async fn record_failure(&self, error: &ZelanError) -> ZelanError {
        // Register the error if an error registry is provided
        let registered_error = if let Some(registry) = &self.error_registry {
            registry.register(error.clone(), Some(&self.name)).await
        } else {
            error.clone()
        };
        
        // Atomically increment the failure count
        let count = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
        
        // Open the circuit if we've reached the threshold
        if count >= self.failure_threshold as usize {
            self.is_open.store(true, Ordering::Release);
            *self.open_time.write().await = Some(Instant::now());
            
            // Log at appropriate level based on error severity
            match registered_error.severity {
                ErrorSeverity::Critical => {
                    error!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened due to critical failures"
                    );
                }
                ErrorSeverity::Error => {
                    warn!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened due to repeated errors"
                    );
                }
                _ => {
                    info!(
                        name = self.name,
                        failure_count = count,
                        threshold = self.failure_threshold,
                        reset_timeout_ms = self.reset_timeout.as_millis(),
                        "Circuit breaker opened"
                    );
                }
            }
        }
        
        registered_error
    }
    
    /// Execute a function with circuit breaker protection
    pub async fn execute<F, Fut, T>(&self, f: F) -> ZelanResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ZelanResult<T>>,
    {
        // Check if circuit is open
        if self.is_open().await {
            return Err(ZelanError::new(ErrorCode::Internal)
                .message(format!("Circuit breaker '{}' is open", self.name))
                .context("Too many failures")
                .severity(ErrorSeverity::Warning)
                .category(ErrorCategory::ServiceUnavailable)
                .build());
        }
        
        // Execute the function
        match f().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(err) => {
                // Record the failure and return the error
                let registered_err = self.record_failure(&err).await;
                Err(registered_err)
            }
        }
    }
    
    /// Execute a function with retry and circuit breaker protection
    pub async fn execute_with_retry<F, Fut, T>(&self, operation_name: &str, retry_policy: &RetryPolicy, f: F) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        let f_clone = f.clone();
        
        // Use the circuit breaker to protect the retry logic
        self.execute(|| async move {
            with_retry(operation_name, retry_policy, f_clone).await
        }).await
    }
}

/// Simplified recovery manager using our own circuit breaker
#[derive(Clone)]
pub struct SimpleRecoveryManager {
    /// Error registry for tracking errors
    error_registry: Arc<ErrorRegistry>,
    /// Circuit breakers for different operations
    circuit_breakers: Arc<RwLock<HashMap<String, SimpleCircuitBreaker>>>,
    /// Default retry policies for different error categories
    default_policies: Arc<HashMap<ErrorCategory, RetryPolicy>>,
}

impl SimpleRecoveryManager {
    /// Create a new recovery manager with default settings
    pub fn new() -> Self {
        let error_registry = Arc::new(ErrorRegistry::new(1000));
        
        // Set up default retry policies
        let mut default_policies = HashMap::new();
        default_policies.insert(
            ErrorCategory::Network,
            RetryPolicy::exponential_backoff(
                5,
                Duration::from_millis(500),
                2.0,
                Some(Duration::from_secs(30)),
            ),
        );
        default_policies.insert(
            ErrorCategory::Authentication,
            RetryPolicy::fixed_delay(2, Duration::from_secs(2)),
        );
        default_policies.insert(
            ErrorCategory::RateLimit,
            RetryPolicy::exponential_backoff(
                3,
                Duration::from_secs(1),
                2.0,
                Some(Duration::from_secs(60)),
            ),
        );
        default_policies.insert(
            ErrorCategory::ServiceUnavailable,
            RetryPolicy::exponential_backoff(
                5,
                Duration::from_secs(5),
                2.0,
                Some(Duration::from_secs(120)),
            ),
        );

        Self {
            error_registry,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            default_policies: Arc::new(default_policies),
        }
    }

    /// Get the error registry
    pub fn error_registry(&self) -> Arc<ErrorRegistry> {
        self.error_registry.clone()
    }

    /// Register an error with the registry
    pub async fn register_error(&self, error: ZelanError, source: Option<&str>) -> ZelanError {
        self.error_registry.register(error, source).await
    }

    /// Get or create a circuit breaker for a specific operation
    pub async fn get_circuit_breaker(&self, name: &str) -> SimpleCircuitBreaker {
        let mut breakers = self.circuit_breakers.write().await;
        
        if let Some(breaker) = breakers.get(name) {
            return breaker.clone();
        }
        
        // Create a new circuit breaker with default settings
        let breaker = SimpleCircuitBreaker::new(name, 5, Duration::from_secs(60))
            .with_error_registry(self.error_registry.clone());
        
        breakers.insert(name.to_string(), breaker.clone());
        breaker
    }

    /// Create a circuit breaker with custom settings
    pub async fn create_circuit_breaker(
        &self,
        name: &str,
        failure_threshold: u32,
        reset_timeout: Duration,
    ) -> SimpleCircuitBreaker {
        let breaker = SimpleCircuitBreaker::new(name, failure_threshold, reset_timeout)
            .with_error_registry(self.error_registry.clone());
        
        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(name.to_string(), breaker.clone());
        
        breaker
    }

    /// Get the default retry policy for an error category
    pub fn get_default_policy(&self, category: ErrorCategory) -> RetryPolicy {
        self.default_policies
            .get(&category)
            .cloned()
            .unwrap_or_else(|| category.default_retry_policy())
    }

    /// Execute a function with automatic retry based on error category
    pub async fn with_auto_retry<F, Fut, T>(
        &self,
        operation_name: &str,
        f: F,
    ) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        // First attempt without retry to get the error category
        match f().await {
            Ok(value) => Ok(value),
            Err(err) => {
                // Register the error
                let registered_err = self.error_registry.register(err, Some(operation_name)).await;
                
                // Determine retry policy based on error category
                if let Some(category) = registered_err.category {
                    if category.is_retryable() {
                        let policy = self.get_default_policy(category);
                        
                        // If retryable, use the policy to retry
                        debug!(
                            operation = operation_name,
                            category = ?category,
                            max_retries = policy.max_retries,
                            "Using automatic retry for operation"
                        );
                        
                        return with_retry(operation_name, &policy, f).await;
                    }
                }
                
                // Not retryable or no category
                Err(registered_err)
            }
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn with_circuit_breaker<F, Fut, T>(
        &self,
        operation_name: &str,
        f: F,
    ) -> ZelanResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ZelanResult<T>>,
    {
        let breaker = self.get_circuit_breaker(operation_name).await;
        breaker.execute(f).await
    }

    /// Execute a function with both circuit breaker and retry
    pub async fn with_protection<F, Fut, T>(
        &self,
        operation_name: &str,
        f: F,
    ) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        let breaker = self.get_circuit_breaker(operation_name).await;
        let f_clone = f.clone();
        
        // First attempt to get the error category for retries
        match f().await {
            Ok(value) => Ok(value),
            Err(err) => {
                // Register the error
                let registered_err = self.error_registry.register(err, Some(operation_name)).await;
                
                // Determine retry policy based on error category
                if let Some(category) = registered_err.category {
                    if category.is_retryable() {
                        let policy = self.get_default_policy(category);
                        
                        // If retryable, use the policy to retry with circuit breaker
                        debug!(
                            operation = operation_name,
                            category = ?category,
                            max_retries = policy.max_retries,
                            "Using automatic retry with circuit breaker for operation"
                        );
                        
                        return breaker.execute_with_retry(operation_name, &policy, f_clone).await;
                    }
                }
                
                // Not retryable or no category
                Err(registered_err)
            }
        }
    }
}

/// Simplified adapter recovery trait
pub trait SimpleAdapterRecovery {
    /// Get the recovery manager
    fn recovery_manager(&self) -> Arc<SimpleRecoveryManager>;
    
    /// Get the adapter name for error registration
    fn adapter_name(&self) -> &str;
    
    /// Execute an adapter operation with protection
    #[allow(async_fn_in_trait)]
    async fn protected_operation<F, Fut, T>(&self, operation_name: &str, f: F) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        // Create the full operation name with adapter prefix
        let full_operation = format!("{}.{}", self.adapter_name(), operation_name);
        
        // Use the recovery manager to execute with protection
        self.recovery_manager().with_protection(&full_operation, f).await
    }
    
    /// Register an adapter-specific error
    #[allow(async_fn_in_trait)]
    async fn register_error(&self, error: ZelanError) -> ZelanError {
        let adapter_name = self.adapter_name();
        self.recovery_manager().register_error(error, Some(adapter_name)).await
    }
}

/*
Example of using the simplified circuit breaker and recovery system:

```rust
use std::sync::Arc;

// Define an adapter that implements the SimpleAdapterRecovery trait
struct MyAdapter {
    name: String,
    recovery_manager: Arc<SimpleRecoveryManager>,
}

impl MyAdapter {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            recovery_manager: Arc::new(SimpleRecoveryManager::new()),
        }
    }
    
    // Example API call with retry and circuit breaker protection
    async fn get_data(&self, id: &str) -> ZelanResult<String> {
        self.protected_operation("get_data", || async {
            // Simulated API call that might fail
            self.api_request(id).await
        }).await
    }
    
    // Low-level API function that might fail
    async fn api_request(&self, id: &str) -> ZelanResult<String> {
        // Simulate occasional failures
        if id == "error" {
            return Err(ZelanError::new(ErrorCode::NetworkTimeout)
                      .message("Network timeout")
                      .context("API request")
                      .category(ErrorCategory::Network)
                      .build());
        }
        
        Ok(format!("Data for {}", id))
    }
}

impl SimpleAdapterRecovery for MyAdapter {
    fn recovery_manager(&self) -> Arc<SimpleRecoveryManager> {
        self.recovery_manager.clone()
    }
    
    fn adapter_name(&self) -> &str {
        &self.name
    }
}

// Usage example
async fn main() {
    let adapter = MyAdapter::new("example");
    
    // Will automatically retry on network errors and use circuit breaker
    match adapter.get_data("12345").await {
        Ok(data) => println!("Got data: {}", data),
        Err(err) => println!("Error: {}", err),
    }
}
```
*/
