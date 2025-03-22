//! Service adapter implementations

mod base;
mod http_client;
mod obs;
mod test;
mod twitch;
mod twitch_api;
mod twitch_auth;
mod twitch_eventsub;

// Export the adapter trait and implementations
// These are just placeholders for now, we'll implement them properly later
// pub use base::BaseAdapter;
// pub use obs::ObsAdapter;
// pub use test::TestAdapter;
// pub use twitch::TwitchAdapter;

// Re-export the HTTP client for use in adapters
pub use http_client::{HttpClient, MockHttpClient, ReqwestHttpClient, SimpleHttpResponse};

// Export trait for adapter implementations
use async_trait::async_trait;
use serde_json::Value;
use std::fmt::Debug;
use anyhow::Result;

/// Trait for service adapters that can connect to external services
#[async_trait]
pub trait ServiceAdapter: Send + Sync + Debug {
    /// Connect to the service
    async fn connect(&self) -> Result<()>;

    /// Disconnect from the service
    async fn disconnect(&self) -> Result<()>;

    /// Check if the adapter is currently connected
    fn is_connected(&self) -> bool;

    /// Get the adapter's name
    fn get_name(&self) -> &str;

    /// Set configuration for the adapter (optional)
    async fn configure(&self, _config: Value) -> Result<()> {
        Ok(()) // Default implementation does nothing
    }
}