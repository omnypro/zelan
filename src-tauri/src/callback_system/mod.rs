//! Standardized callback management system
//! 
//! This module provides a robust way to manage callbacks across clone boundaries
//! and async contexts. It solves issues with callbacks not being preserved when
//! adapters are cloned.
//!
//! See the README.md file for usage examples and best practices.

#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use uuid::Uuid;

/// Type for callback IDs
pub type CallbackId = Uuid;

/// Trait for types that can be used in callbacks
pub trait CallbackData: Clone + Send + Sync + 'static + fmt::Debug {}

// Implement CallbackData for common types
impl<T> CallbackData for T where T: Clone + Send + Sync + 'static + fmt::Debug {}

/// A container for callbacks that ensures they're preserved across clone boundaries
#[derive(Clone)]
pub struct CallbackRegistry<T: CallbackData> {
    /// The shared registry of callbacks
    callbacks: Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(T) -> Result<()> + Send + Sync + 'static>>>>,
    
    /// Optional group identifier for categorizing callbacks
    group: Option<String>,
}

impl<T: CallbackData> CallbackRegistry<T> {
    /// Create a new callback registry
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            group: None,
        }
    }
    
    /// Create a new callback registry with a group identifier
    pub fn with_group(group: &str) -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            group: Some(group.to_string()),
        }
    }
    
    /// Register a callback function
    pub async fn register<F>(&self, callback: F) -> CallbackId 
    where
        F: Fn(T) -> Result<()> + Send + Sync + 'static,
    {
        let id = Uuid::new_v4();
        let mut callbacks = self.callbacks.write().await;
        callbacks.insert(id, Box::new(callback));
        
        if let Some(group) = &self.group {
            tracing::debug!(
                callback_id = %id,
                group = %group,
                "Registered callback"
            );
        } else {
            tracing::debug!(
                callback_id = %id,
                "Registered callback"
            );
        }
        
        id
    }
    
    /// Unregister a callback by ID
    pub async fn unregister(&self, id: CallbackId) -> bool {
        let mut callbacks = self.callbacks.write().await;
        let removed = callbacks.remove(&id).is_some();
        
        if removed {
            if let Some(group) = &self.group {
                tracing::debug!(
                    callback_id = %id,
                    group = %group,
                    "Unregistered callback"
                );
            } else {
                tracing::debug!(
                    callback_id = %id,
                    "Unregistered callback"
                );
            }
        } else {
            if let Some(group) = &self.group {
                tracing::warn!(
                    callback_id = %id,
                    group = %group,
                    "Attempted to unregister non-existent callback"
                );
            } else {
                tracing::warn!(
                    callback_id = %id,
                    "Attempted to unregister non-existent callback"
                );
            }
        }
        
        removed
    }
    
    /// Trigger all registered callbacks with the provided data
    pub async fn trigger(&self, data: T) -> Result<usize> {
        let callbacks = self.callbacks.read().await;
        let callback_count = callbacks.len();
        
        if callback_count == 0 {
            // No callbacks registered, just return
            return Ok(0);
        }
        
        let mut success_count = 0;
        let mut errors = Vec::new();
        
        // Call each callback and collect errors
        for (id, callback) in callbacks.iter() {
            match callback(data.clone()) {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    let error_msg = format!("Callback {} failed: {}", id, e);
                    tracing::error!(callback_id = %id, error = %e, "Callback execution failed");
                    errors.push(error_msg);
                }
            }
        }
        
        // Log summary
        if let Some(group) = &self.group {
            if errors.is_empty() {
                tracing::debug!(
                    group = %group,
                    count = callback_count,
                    "All callbacks executed successfully"
                );
            } else {
                tracing::warn!(
                    group = %group,
                    success = success_count,
                    errors = errors.len(),
                    "Some callbacks failed"
                );
            }
        } else {
            if errors.is_empty() {
                tracing::debug!(
                    count = callback_count,
                    "All callbacks executed successfully"
                );
            } else {
                tracing::warn!(
                    success = success_count,
                    errors = errors.len(),
                    "Some callbacks failed"
                );
            }
        }
        
        // If any callbacks failed, return an error with details
        if !errors.is_empty() {
            return Err(anyhow!("{} of {} callbacks failed: {}", 
                errors.len(), callback_count, errors.join("; ")));
        }
        
        Ok(success_count)
    }
    
    /// Get the number of registered callbacks
    pub async fn count(&self) -> usize {
        self.callbacks.read().await.len()
    }
    
    /// Clear all registered callbacks
    pub async fn clear(&self) {
        let mut callbacks = self.callbacks.write().await;
        let count = callbacks.len();
        callbacks.clear();
        
        if let Some(group) = &self.group {
            tracing::debug!(
                group = %group,
                count = count,
                "Cleared all callbacks"
            );
        } else {
            tracing::debug!(
                count = count,
                "Cleared all callbacks"
            );
        }
    }
}

/// Provides an easier way to manage multiple callback registries
#[derive(Clone, Default)]
pub struct CallbackManager {
    /// Registries by name
    registries: Arc<RwLock<HashMap<String, Box<dyn CallbackRegistryAny + Send + Sync>>>>,
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
        Box::pin(async move {
            self_clone.count().await
        })
    }
}

impl CallbackManager {
    /// Create a new callback manager
    pub fn new() -> Self {
        Self {
            registries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get or create a callback registry for a specific type and name
    pub async fn registry<T: CallbackData>(&self, name: &str) -> CallbackRegistry<T> {
        let mut registries = self.registries.write().await;
        
        // Check if registry exists already
        if let Some(registry) = registries.get(name) {
            // Verify the type matches
            if registry.type_name() == std::any::type_name::<T>() {
                // Use unsafe to downcast because we've verified the type
                // This is safe because we've checked the type matches
                let registry_ref = registries.get(name).unwrap();
                let registry_box = unsafe {
                    &*(registry_ref as *const _ as *const CallbackRegistry<T>)
                };
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
        registries.insert(name.to_string(), Box::new(registry.clone()));
        
        registry
    }
    
    /// Get all registry names
    pub async fn registry_names(&self) -> Vec<String> {
        let registries = self.registries.read().await;
        registries.keys().cloned().collect()
    }
    
    /// Get the number of callbacks in a specific registry
    pub async fn registry_count(&self, name: &str) -> Result<usize> {
        let registries = self.registries.read().await;
        
        if let Some(registry) = registries.get(name) {
            Ok(registry.count_async().await)
        } else {
            Err(anyhow!("Registry '{}' not found", name))
        }
    }
    
    /// Get statistics about all registries
    pub async fn stats(&self) -> HashMap<String, usize> {
        let registries = self.registries.read().await;
        let mut stats = HashMap::new();
        
        for (name, registry) in registries.iter() {
            let count = registry.count_async().await;
            stats.insert(name.clone(), count);
        }
        
        stats
    }
}