use serde::{Deserialize, Serialize};
use std::fmt;

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
}

/// Error codes for different types of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    // Configuration related errors
    ConfigInvalid,
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
            ErrorCode::ConfigInvalid => write!(f, "CONFIG_INVALID"),
        }
    }
}

impl std::error::Error for ZelanError {}

impl From<anyhow::Error> for ZelanError {
    fn from(err: anyhow::Error) -> Self {
        ZelanError {
            code: ErrorCode::Internal,
            message: err.to_string(),
            context: None,
            severity: ErrorSeverity::Error,
        }
    }
}

// Custom Result type for Zelan operations
pub type ZelanResult<T> = Result<T, ZelanError>;

// Utility functions to create errors
pub fn adapter_not_found(name: &str) -> ZelanError {
    ZelanError {
        code: ErrorCode::AdapterNotFound,
        message: format!("Adapter '{}' not found", name),
        context: None,
        severity: ErrorSeverity::Error,
    }
}

pub fn adapter_connection_failed(name: &str, err: impl fmt::Display) -> ZelanError {
    ZelanError {
        code: ErrorCode::AdapterConnectionFailed,
        message: format!("Failed to connect adapter '{}'", name),
        context: Some(err.to_string()),
        severity: ErrorSeverity::Error,
    }
}

pub fn event_bus_publish_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError {
        code: ErrorCode::EventBusPublishFailed,
        message: "Failed to publish event to event bus".to_string(),
        context: Some(err.to_string()),
        severity: ErrorSeverity::Error,
    }
}

pub fn event_bus_dropped() -> ZelanError {
    ZelanError {
        code: ErrorCode::EventBusDropped,
        message: "Event was dropped due to no receivers".to_string(),
        context: None,
        severity: ErrorSeverity::Warning,
    }
}

pub fn websocket_bind_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError {
        code: ErrorCode::WebSocketBindFailed,
        message: "Failed to bind WebSocket server".to_string(),
        context: Some(err.to_string()),
        severity: ErrorSeverity::Critical,
    }
}

pub fn websocket_accept_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError {
        code: ErrorCode::WebSocketAcceptFailed,
        message: "Failed to accept WebSocket connection".to_string(),
        context: Some(err.to_string()),
        severity: ErrorSeverity::Error,
    }
}

pub fn websocket_send_failed(err: impl fmt::Display) -> ZelanError {
    ZelanError {
        code: ErrorCode::WebSocketSendFailed,
        message: "Failed to send WebSocket message".to_string(),
        context: Some(err.to_string()),
        severity: ErrorSeverity::Error,
    }
}
