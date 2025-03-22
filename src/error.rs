use std::fmt;
use thiserror::Error;

/// Main error type for the Zelan application
#[derive(Error, Debug, Clone)]
pub struct ZelanError {
    /// Error code for categorization and identification
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional context for more detailed error information
    pub context: Option<String>,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Optional category for filtering and handling
    pub category: Option<ErrorCategory>,
    /// Optional unique error ID
    pub error_id: Option<String>,
}

/// Type alias for Zelan result
pub type ZelanResult<T> = Result<T, ZelanError>;

impl fmt::Display for ZelanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)?;
        if let Some(context) = &self.context {
            write!(f, " ({})", context)?;
        }
        Ok(())
    }
}

/// Enumeration of error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // General errors
    Unknown,
    ConfigInvalid,
    
    // Adapter errors
    AdapterNotFound,
    AdapterConnectionFailed,
    AdapterDisconnectFailed,
    AdapterAuthenticationFailed,
    AdapterDisabled,
    
    // WebSocket errors
    WebSocketBindFailed,
    WebSocketAcceptFailed,
    WebSocketSendFailed,
    
    // Event bus errors
    EventBusPublishFailed,
    
    // Authentication errors
    AuthTokenInvalid,
    AuthTokenExpired,
    AuthRefreshFailed,
    AuthorizationFailed,
    
    // API errors
    ApiRequestFailed,
    ApiResponseInvalid,
    ApiRateLimited,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code_str = match self {
            // General errors
            ErrorCode::Unknown => "UNKNOWN",
            ErrorCode::ConfigInvalid => "CONFIG_INVALID",
            
            // Adapter errors
            ErrorCode::AdapterNotFound => "ADAPTER_NOT_FOUND",
            ErrorCode::AdapterConnectionFailed => "ADAPTER_CONNECTION_FAILED",
            ErrorCode::AdapterDisconnectFailed => "ADAPTER_DISCONNECT_FAILED",
            ErrorCode::AdapterAuthenticationFailed => "ADAPTER_AUTH_FAILED",
            ErrorCode::AdapterDisabled => "ADAPTER_DISABLED",
            
            // WebSocket errors
            ErrorCode::WebSocketBindFailed => "WS_BIND_FAILED",
            ErrorCode::WebSocketAcceptFailed => "WS_ACCEPT_FAILED",
            ErrorCode::WebSocketSendFailed => "WS_SEND_FAILED",
            
            // Event bus errors
            ErrorCode::EventBusPublishFailed => "EVENT_BUS_PUBLISH_FAILED",
            
            // Authentication errors
            ErrorCode::AuthTokenInvalid => "AUTH_TOKEN_INVALID",
            ErrorCode::AuthTokenExpired => "AUTH_TOKEN_EXPIRED",
            ErrorCode::AuthRefreshFailed => "AUTH_REFRESH_FAILED",
            ErrorCode::AuthorizationFailed => "AUTH_FAILED",
            
            // API errors
            ErrorCode::ApiRequestFailed => "API_REQUEST_FAILED",
            ErrorCode::ApiResponseInvalid => "API_RESPONSE_INVALID",
            ErrorCode::ApiRateLimited => "API_RATE_LIMITED",
        };
        write!(f, "{}", code_str)
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational only, not an actual error
    Info,
    /// Warning that doesn't prevent operation
    Warning,
    /// Error that affects functionality but allows continued operation
    Error,
    /// Severe error that prevents further operation
    Critical,
}

/// Error category for filtering and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (connection, timeout, etc.)
    Network,
    /// Authentication-related errors
    Authentication,
    /// Configuration-related errors
    Configuration,
    /// External service errors
    Service,
    /// Internal errors
    Internal,
}

/// Retry policy for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicy {
    /// Don't retry
    NoRetry,
    /// Retry with exponential backoff
    Exponential,
    /// Retry with fixed interval
    Fixed,
    /// Retry immediately
    Immediate,
}

// Helper functions to create standard errors

/// Create an adapter not found error
pub fn adapter_not_found(name: &str) -> ZelanError {
    ZelanError {
        code: ErrorCode::AdapterNotFound,
        message: format!("Adapter '{}' not found", name),
        context: None,
        severity: ErrorSeverity::Error,
        category: Some(ErrorCategory::Internal),
        error_id: None,
    }
}

/// Create an adapter connection failed error
pub fn adapter_connection_failed(name: &str, error: &anyhow::Error) -> ZelanError {
    ZelanError {
        code: ErrorCode::AdapterConnectionFailed,
        message: format!("Failed to connect adapter '{}'", name),
        context: Some(error.to_string()),
        severity: ErrorSeverity::Error,
        category: Some(ErrorCategory::Network),
        error_id: None,
    }
}

/// Create a WebSocket accept failed error
pub fn websocket_accept_failed(error: impl std::error::Error) -> ZelanError {
    ZelanError {
        code: ErrorCode::WebSocketAcceptFailed,
        message: "WebSocket accept failed".to_string(),
        context: Some(error.to_string()),
        severity: ErrorSeverity::Warning,
        category: Some(ErrorCategory::Network),
        error_id: None,
    }
}

/// Create an event bus publish failed error
pub fn event_bus_publish_failed(error: impl std::error::Error) -> ZelanError {
    ZelanError {
        code: ErrorCode::EventBusPublishFailed,
        message: "Failed to publish event to event bus".to_string(),
        context: Some(error.to_string()),
        severity: ErrorSeverity::Warning,
        category: Some(ErrorCategory::Internal),
        error_id: None,
    }
}