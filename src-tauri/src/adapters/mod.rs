use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::events::{EventBus, StreamEvent};

// Re-export specific adapters
pub mod twitch;

/// Status of a service adapter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterStatus {
    /// Adapter is disconnected from the service
    Disconnected,
    /// Adapter is in the process of connecting
    Connecting,
    /// Adapter is connected to the service
    Connected,
    /// Adapter encountered an error
    Error,
    /// Adapter is disabled
    Disabled,
}

/// Common trait for all service adapters
#[async_trait]
pub trait ServiceAdapter: Send + Sync {
    /// Get the adapter name
    fn name(&self) -> &str;
    
    /// Connect to the service
    async fn connect(&self) -> Result<()>;
    
    /// Disconnect from the service
    async fn disconnect(&self) -> Result<()>;
    
    /// Check if the adapter is connected
    fn is_connected(&self) -> bool;
    
    /// Configure the adapter with settings
    async fn configure(&self, config: serde_json::Value) -> Result<()>;
    
    /// Get adapter metrics
    async fn get_metrics(&self) -> Result<serde_json::Value> {
        // Default implementation returns empty object
        Ok(serde_json::json!({}))
    }
}

/// Settings for an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterSettings {
    /// Whether the adapter is enabled
    pub enabled: bool,
    /// Adapter-specific configuration
    #[serde(default)]
    pub config: serde_json::Value,
    /// Display name for the adapter
    pub display_name: String,
    /// Description of the adapter's functionality
    pub description: String,
}

/// Base implementation for adapters with common functionality
pub struct BaseAdapter {
    name: String,
    event_bus: Arc<EventBus>,
    status: Arc<RwLock<AdapterStatus>>,
}

impl BaseAdapter {
    /// Create a new base adapter
    pub fn new(name: &str, event_bus: Arc<EventBus>) -> Self {
        Self {
            name: name.to_string(),
            event_bus,
            status: Arc::new(RwLock::new(AdapterStatus::Disconnected)),
        }
    }
    
    /// Get the adapter name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the current status
    pub async fn status(&self) -> AdapterStatus {
        *self.status.read().await
    }
    
    /// Set the adapter status
    pub async fn set_status(&self, status: AdapterStatus) {
        *self.status.write().await = status;
    }
    
    /// Publish an event to the event bus
    pub async fn publish_event(&self, event_type: &str, payload: serde_json::Value) -> Result<()> {
        let event = StreamEvent::new(&self.name, event_type, payload);
        
        match self.event_bus.publish(event).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Failed to publish event: {}", e)),
        }
    }
}

/// Manager for all adapters
pub struct AdapterManager {
    event_bus: Arc<EventBus>,
    adapters: Arc<RwLock<HashMap<String, Arc<Box<dyn ServiceAdapter>>>>>,
    settings: Arc<RwLock<HashMap<String, AdapterSettings>>>,
    status: Arc<RwLock<HashMap<String, AdapterStatus>>>,
}

impl AdapterManager {
    /// Create a new adapter manager
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an adapter with the manager
    pub async fn register_adapter<A>(&self, adapter: A, settings: Option<AdapterSettings>)
    where
        A: ServiceAdapter + 'static,
    {
        let name = adapter.name().to_string();
        info!(adapter = %name, "Registering adapter");
        
        // Box and store the adapter
        let adapter_box = Arc::new(Box::new(adapter) as Box<dyn ServiceAdapter>);
        
        // Store the adapter
        self.adapters
            .write()
            .await
            .insert(name.clone(), adapter_box);
        
        // Default settings if none provided
        let adapter_settings = settings.unwrap_or_else(|| AdapterSettings {
            enabled: true,
            config: serde_json::Value::Null,
            display_name: name.clone(),
            description: format!("{} adapter", name),
        });
        
        // Store settings
        self.settings
            .write()
            .await
            .insert(name.clone(), adapter_settings);
        
        // Set initial status
        let initial_status = if self.settings.read().await.get(&name).unwrap().enabled {
            AdapterStatus::Disconnected
        } else {
            AdapterStatus::Disabled
        };
        
        self.status.write().await.insert(name, initial_status);
    }
    
    /// Connect an adapter by name
    pub async fn connect_adapter(&self, name: &str) -> Result<()> {
        debug!(adapter = %name, "Connecting adapter");
        
        // Check if adapter exists
        let adapter = match self.adapters.read().await.get(name) {
            Some(adapter) => adapter.clone(),
            None => return Err(anyhow!("Adapter '{}' not found", name)),
        };
        
        // Check if adapter is enabled
        let settings = self.settings.read().await.get(name).cloned();
        let is_enabled = settings.map_or(true, |s| s.enabled);
        
        if !is_enabled {
            return Err(anyhow!("Adapter '{}' is disabled", name));
        }
        
        // Update status to connecting
        self.status
            .write()
            .await
            .insert(name.to_string(), AdapterStatus::Connecting);
        
        // Attempt to connect
        match adapter.connect().await {
            Ok(()) => {
                // Update status to connected
                self.status
                    .write()
                    .await
                    .insert(name.to_string(), AdapterStatus::Connected);
                
                info!(adapter = %name, "Successfully connected adapter");
                Ok(())
            }
            Err(e) => {
                // Update status to error
                self.status
                    .write()
                    .await
                    .insert(name.to_string(), AdapterStatus::Error);
                
                error!(adapter = %name, error = %e, "Failed to connect adapter");
                Err(anyhow!("Failed to connect adapter '{}': {}", name, e))
            }
        }
    }
    
    /// Disconnect an adapter by name
    pub async fn disconnect_adapter(&self, name: &str) -> Result<()> {
        debug!(adapter = %name, "Disconnecting adapter");
        
        // Check if adapter exists
        let adapter = match self.adapters.read().await.get(name) {
            Some(adapter) => adapter.clone(),
            None => return Err(anyhow!("Adapter '{}' not found", name)),
        };
        
        // Attempt to disconnect
        match adapter.disconnect().await {
            Ok(()) => {
                // Update status to disconnected
                self.status
                    .write()
                    .await
                    .insert(name.to_string(), AdapterStatus::Disconnected);
                
                info!(adapter = %name, "Successfully disconnected adapter");
                Ok(())
            }
            Err(e) => {
                // Update status but don't change - might be partially connected
                error!(adapter = %name, error = %e, "Failed to disconnect adapter");
                Err(anyhow!("Failed to disconnect adapter '{}': {}", name, e))
            }
        }
    }
    
    /// Connect all enabled adapters
    pub async fn connect_all(&self) -> Result<()> {
        info!("Connecting all enabled adapters");
        
        let adapter_names = {
            let adapters = self.adapters.read().await;
            adapters.keys().cloned().collect::<Vec<_>>()
        };
        
        for name in adapter_names {
            let settings = self.settings.read().await.get(&name).cloned();
            let is_enabled = settings.map_or(true, |s| s.enabled);
            
            if is_enabled {
                if let Err(e) = self.connect_adapter(&name).await {
                    error!(adapter = %name, error = %e, "Failed to connect adapter during connect_all");
                    // Continue with other adapters
                }
            }
        }
        
        Ok(())
    }
    
    /// Update adapter settings
    pub async fn update_adapter_settings(&self, name: &str, settings: AdapterSettings) -> Result<()> {
        debug!(adapter = %name, "Updating adapter settings");
        
        // Check if adapter exists
        if !self.adapters.read().await.contains_key(name) {
            return Err(anyhow!("Adapter '{}' not found", name));
        }
        
        // Get old settings
        let old_enabled = self.settings.read().await.get(name).map_or(true, |s| s.enabled);
        
        // Update settings
        self.settings
            .write()
            .await
            .insert(name.to_string(), settings.clone());
        
        // Handle enable/disable changes
        if old_enabled != settings.enabled {
            if settings.enabled {
                // Changed from disabled to enabled
                self.status
                    .write()
                    .await
                    .insert(name.to_string(), AdapterStatus::Disconnected);
            } else {
                // Changed from enabled to disabled
                let current_status = self.status.read().await.get(name).copied();
                
                if let Some(status) = current_status {
                    if status == AdapterStatus::Connected || status == AdapterStatus::Connecting {
                        // Disconnect if connected
                        if let Err(e) = self.disconnect_adapter(name).await {
                            error!(adapter = %name, error = %e, "Failed to disconnect adapter when disabling");
                        }
                    }
                    
                    // Set status to disabled
                    self.status
                        .write()
                        .await
                        .insert(name.to_string(), AdapterStatus::Disabled);
                }
            }
        }
        
        // Apply configuration if provided
        if !settings.config.is_null() {
            if let Some(adapter) = self.adapters.read().await.get(name) {
                if let Err(e) = adapter.configure(settings.config.clone()).await {
                    return Err(anyhow!("Failed to configure adapter '{}': {}", name, e));
                }
            }
        }
        
        Ok(())
    }
    
    /// Get adapter status
    pub async fn get_adapter_status(&self, name: &str) -> Result<AdapterStatus> {
        match self.status.read().await.get(name) {
            Some(status) => Ok(*status),
            None => Err(anyhow!("Adapter '{}' not found", name)),
        }
    }
    
    /// Get all adapter statuses
    pub async fn get_all_statuses(&self) -> HashMap<String, AdapterStatus> {
        self.status.read().await.clone()
    }
    
    /// Get adapter settings
    pub async fn get_adapter_settings(&self, name: &str) -> Result<AdapterSettings> {
        match self.settings.read().await.get(name) {
            Some(settings) => Ok(settings.clone()),
            None => Err(anyhow!("Adapter '{}' not found", name)),
        }
    }
    
    /// Get all adapter settings
    pub async fn get_all_settings(&self) -> HashMap<String, AdapterSettings> {
        self.settings.read().await.clone()
    }
    
    /// Get adapter metrics
    pub async fn get_adapter_metrics(&self, name: &str) -> Result<serde_json::Value> {
        match self.adapters.read().await.get(name) {
            Some(adapter) => adapter.get_metrics().await,
            None => Err(anyhow!("Adapter '{}' not found", name)),
        }
    }
    
    /// Get the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }
}