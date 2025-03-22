//! WebSocket server implementation
//!
//! This module provides WebSocket server functionality for streaming events
//! to external clients in real-time.

mod server;
mod client_preferences;

// Re-export main components
pub use server::WebSocketServer;
pub use client_preferences::WebSocketClientPreferences;

/// Configuration for the WebSocket server
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebSocketConfig {
    /// Port to bind the WebSocket server to
    pub port: u16,
    /// Maximum number of simultaneous connections
    #[serde(default = "crate::core::default_max_connections")]
    pub max_connections: usize,
    /// Inactivity timeout in seconds
    #[serde(default = "crate::core::default_timeout")]
    pub timeout_seconds: u64,
    /// Ping interval in seconds
    #[serde(default = "crate::core::default_ping_interval")]
    pub ping_interval: u64,
}