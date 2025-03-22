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
pub use base::{AdapterConfig, BaseAdapter};
pub use test::TestAdapter;
pub use twitch::TwitchAdapter;
// pub use obs::ObsAdapter;  // Will be implemented later

// Re-export the HTTP client for use in adapters
pub use http_client::{HttpClient, MockHttpClient, ReqwestHttpClient, SimpleHttpResponse};

// Export trait for adapter implementations
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

/// Status of a service adapter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceAdapterStatus {
    /// Adapter is connected and operational
    Connected,
    /// Adapter is disconnected
    Disconnected,
    /// Adapter encountered an error
    Error,
    /// Adapter is connecting
    Connecting,
    /// Adapter is disconnecting
    Disconnecting,
}

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
    
    /// Get the status of the adapter
    async fn status(&self) -> Result<ServiceAdapterStatus> {
        if self.is_connected() {
            Ok(ServiceAdapterStatus::Connected)
        } else {
            Ok(ServiceAdapterStatus::Disconnected)
        }
    }
}