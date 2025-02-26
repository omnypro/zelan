use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use tauri::async_runtime::RwLock;
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
