//! Simplified callback management system
//!
//! This module provides a simpler way to manage callbacks using Tokio's broadcast channels.
//! It replaces the previous complex implementation with direct usage of tokio primitives.

#[cfg(test)]
mod tests;

use std::{fmt, sync::Arc};
use tokio::sync::broadcast;
use tracing::{debug, error};
use uuid::Uuid;

/// Type for callback IDs
pub type CallbackId = Uuid;

/// Trait for types that can be used in callbacks
pub trait CallbackData: Clone + Send + Sync + 'static + fmt::Debug {}

// Implement CallbackData for common types
impl<T> CallbackData for T where T: Clone + Send + Sync + 'static + fmt::Debug {}

/// A simple wrapper around tokio::sync::broadcast::Sender
#[derive(Clone)]
pub struct CallbackRegistry<T: CallbackData> {
    /// The broadcast sender to distribute events
    sender: broadcast::Sender<T>,

    /// Optional group identifier for categorizing callbacks
    group: Option<String>,

    /// Map to keep track of receivers for cleanup
    receivers: Arc<dashmap::DashMap<CallbackId, broadcast::Receiver<T>>>,
}

impl<T: CallbackData> CallbackRegistry<T> {
    /// Create a new callback registry
    pub fn new() -> Self {
        // Create a broadcast channel with reasonable capacity
        let (sender, _) = broadcast::channel(100);

        Self {
            sender,
            group: None,
            receivers: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Create a new callback registry with a group identifier
    pub fn with_group(group: &str) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            sender,
            group: Some(group.to_string()),
            receivers: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Register a callback function
    pub async fn register<F>(&self, callback: F) -> CallbackId
    where
        F: Fn(T) -> anyhow::Result<()> + Send + Sync + 'static,
    {
        let id = Uuid::new_v4();
        let mut receiver = self.sender.subscribe();
        let callback = Arc::new(callback);

        // Spawn a task to process events for this callback
        let log_group = self.group.clone();
        let receivers_map = Arc::clone(&self.receivers);
        let callback_clone = Arc::clone(&callback);

        // Store the receiver so we can drop it later
        self.receivers.insert(id, self.sender.subscribe());

        // Spawn a task to process events for this callback
        tokio::spawn(async move {
            if let Some(group) = &log_group {
                debug!(
                    callback_id = %id,
                    group = %group,
                    "Started callback listener"
                );
            } else {
                debug!(
                    callback_id = %id,
                    "Started callback listener"
                );
            }
            
            // Task is now fully initialized and ready to receive events

            while let Ok(data) = receiver.recv().await {
                // Call the callback with the received data
                if let Err(e) = callback_clone(data) {
                    if let Some(group) = &log_group {
                        error!(
                            callback_id = %id,
                            group = %group,
                            error = %e,
                            "Callback execution failed"
                        );
                    } else {
                        error!(
                            callback_id = %id,
                            error = %e,
                            "Callback execution failed"
                        );
                    }
                }
            }

            // Receiver was dropped, clean up the map
            receivers_map.remove(&id);

            if let Some(group) = &log_group {
                debug!(
                    callback_id = %id,
                    group = %group,
                    "Callback listener stopped"
                );
            } else {
                debug!(
                    callback_id = %id,
                    "Callback listener stopped"
                );
            }
        });

        if let Some(group) = &self.group {
            debug!(
                callback_id = %id,
                group = %group,
                "Registered callback"
            );
        } else {
            debug!(
                callback_id = %id,
                "Registered callback"
            );
        }

        id
    }

    /// Unregister a callback by ID
    pub async fn unregister(&self, id: CallbackId) -> bool {
        let removed = self.receivers.remove(&id).is_some();

        if removed {
            if let Some(group) = &self.group {
                debug!(
                    callback_id = %id,
                    group = %group,
                    "Unregistered callback"
                );
            } else {
                debug!(
                    callback_id = %id,
                    "Unregistered callback"
                );
            }
        } else {
            if let Some(group) = &self.group {
                debug!(
                    callback_id = %id,
                    group = %group,
                    "Attempted to unregister non-existent callback"
                );
            } else {
                debug!(
                    callback_id = %id,
                    "Attempted to unregister non-existent callback"
                );
            }
        }

        removed
    }

    /// Trigger all registered callbacks with the provided data
    pub async fn trigger(&self, data: T) -> anyhow::Result<usize> {
        let receivers_count = self.receivers.len();

        if receivers_count == 0 {
            // No callbacks registered, just return
            return Ok(0);
        }

        // Send the data to all receivers
        match self.sender.send(data) {
            Ok(count) => {
                if let Some(group) = &self.group {
                    debug!(
                        group = %group,
                        receivers = receivers_count,
                        delivered = count,
                        "Triggered callbacks"
                    );
                } else {
                    debug!(
                        receivers = receivers_count,
                        delivered = count,
                        "Triggered callbacks"
                    );
                }
                Ok(count)
            }
            Err(e) => {
                if let Some(group) = &self.group {
                    error!(
                        group = %group,
                        error = %e,
                        "Failed to trigger callbacks"
                    );
                } else {
                    error!(
                        error = %e,
                        "Failed to trigger callbacks"
                    );
                }
                Err(anyhow::anyhow!("Failed to trigger callbacks: {}", e))
            }
        }
    }

    /// Get the number of registered callbacks
    pub async fn count(&self) -> usize {
        self.receivers.len()
    }

    /// Clear all registered callbacks
    pub async fn clear(&self) {
        let count = self.receivers.len();
        self.receivers.clear();

        if let Some(group) = &self.group {
            debug!(
                group = %group,
                count = count,
                "Cleared all callbacks"
            );
        } else {
            debug!(count = count, "Cleared all callbacks");
        }
    }
}

/// Provides an easier way to manage multiple callback registries
#[derive(Clone, Default)]
pub struct CallbackManager {
    /// Registries by name
    registries: Arc<dashmap::DashMap<String, Arc<dyn CallbackRegistryAny + Send + Sync>>>,
}

/// Type-erased trait for callback registries to enable storing different registry types
pub trait CallbackRegistryAny: Send + Sync {
    /// Get the type name of the callback registry
    fn type_name(&self) -> &'static str;

    /// Get the count of registered callbacks
    fn count_async(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = usize> + Send>>;
}

impl<T: CallbackData> CallbackRegistryAny for CallbackRegistry<T> {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn count_async(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = usize> + Send>> {
        let self_clone = self.clone();
        Box::pin(async move { self_clone.count().await })
    }
}

impl CallbackManager {
    /// Create a new callback manager
    pub fn new() -> Self {
        Self {
            registries: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Get or create a callback registry for a specific type and name
    pub async fn registry<T: CallbackData>(&self, name: &str) -> CallbackRegistry<T> {
        // Check if registry exists already
        if let Some(registry) = self.registries.get(name) {
            // Verify the type matches
            if registry.type_name() == std::any::type_name::<T>() {
                // Use unsafe to downcast because we've verified the type
                // This is safe because we've checked the type matches
                let registry_ref = registry.value();
                let registry_box =
                    unsafe { &*(registry_ref.as_ref() as *const _ as *const CallbackRegistry<T>) };
                return registry_box.clone();
            } else {
                // Type mismatch, log warning and create new registry
                tracing::warn!(
                    name = %name,
                    existing_type = %registry.type_name(),
                    requested_type = %std::any::type_name::<T>(),
                    "Type mismatch for callback registry, creating new one"
                );
            }
        }

        // Create a new registry
        let registry = CallbackRegistry::<T>::with_group(name);

        // Store it as type-erased version
        self.registries
            .insert(name.to_string(), Arc::new(registry.clone()));

        registry
    }

    /// Get all registry names
    pub async fn registry_names(&self) -> Vec<String> {
        self.registries
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get the number of callbacks in a specific registry
    pub async fn registry_count(&self, name: &str) -> anyhow::Result<usize> {
        if let Some(registry) = self.registries.get(name) {
            Ok(registry.count_async().await)
        } else {
            Err(anyhow::anyhow!("Registry '{}' not found", name))
        }
    }

    /// Get statistics about all registries
    pub async fn stats(&self) -> std::collections::HashMap<String, usize> {
        // Create a vector of futures to resolve counts for each registry
        let count_futures: Vec<_> = self
            .registries
            .iter()
            .map(|entry| {
                let name = entry.key().clone();
                let registry = entry.value().clone();
                async move { (name, registry.count_async().await) }
            })
            .collect();

        // Execute all futures and collect results into a HashMap
        futures::future::join_all(count_futures)
            .await
            .into_iter()
            .collect()
    }
}
