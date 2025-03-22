//! Base adapter functionality for all service adapters

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ServiceAdapter;

/// Configuration for an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Whether the adapter is enabled
    pub enabled: bool,
    /// Adapter-specific configuration
    #[serde(default)]
    pub config: Value,
    /// Display name for the adapter
    pub display_name: String,
    /// Description of the adapter's functionality
    pub description: String,
}

/// Base adapter implementation that other adapters can build upon
#[derive(Debug)]
pub struct BaseAdapter {
    /// The name of this adapter
    name: String,
    /// Whether the adapter is connected
    connected: Arc<RwLock<bool>>,
    /// Adapter configuration
    config: Arc<RwLock<AdapterConfig>>,
}

impl BaseAdapter {
    /// Create a new base adapter
    pub fn new(name: &str, config: AdapterConfig) -> Self {
        Self {
            name: name.to_string(),
            connected: Arc::new(RwLock::new(false)),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Set the connected state
    pub fn set_connected(&self, connected: bool) {
        *self.connected.write() = connected;
    }

    /// Get the current configuration
    pub fn get_config(&self) -> AdapterConfig {
        self.config.read().clone()
    }

    /// Update the configuration
    pub fn update_config(&self, config: AdapterConfig) {
        *self.config.write() = config;
    }
    
    /// Get the adapter name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Check if the adapter is connected
    pub fn is_connected(&self) -> bool {
        *self.connected.read()
    }
}

#[async_trait]
impl ServiceAdapter for BaseAdapter {
    async fn connect(&self) -> Result<()> {
        self.set_connected(true);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.set_connected(false);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        *self.connected.read()
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn configure(&self, config: Value) -> Result<()> {
        let mut current_config = self.get_config();
        current_config.config = config;
        self.update_config(current_config);
        Ok(())
    }
}