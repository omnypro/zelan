//! WebSocket server implementation

mod server;
mod client_preferences;

// We'll implement these properly later
// pub use server::WebSocketServer;
// pub use client_preferences::WebSocketClientPreferences;

/// Configuration for the WebSocket server
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebSocketConfig {
    /// Port to bind the WebSocket server to
    pub port: u16,
    /// Maximum number of simultaneous connections
    pub max_connections: usize,
    /// Inactivity timeout in seconds
    pub timeout_seconds: u64,
    /// Ping interval in seconds
    pub ping_interval: u64,
}