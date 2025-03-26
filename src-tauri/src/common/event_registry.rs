//! Type-safe event registry system for reactive applications
//! 
//! This module provides a clean way to publish and subscribe to events with proper
//! type checking, error handling, and strong guarantees about event delivery.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use thiserror::Error;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

use super::shared_state::LockError;
use super::shared_state::SharedState;

/// Error type for event registry operations
#[derive(Error, Debug, Clone)]
pub enum EventRegistryError {
    /// The event type is not registered with this registry
    #[error("Event type not registered")]
    TypeNotRegistered,
    
    /// Failed to acquire lock on a shared resource
    #[error("Lock error: {0}")]
    LockError(#[from] LockError),
    
    /// Error in the callback execution
    #[error("Callback error: {message}")]
    CallbackError {
        /// Error message
        message: String,
        /// Additional context
        context: Option<String>,
    },
    
    /// Event type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type name
        expected: String,
        /// Actual type name
        actual: String,
    },
}

impl EventRegistryError {
    /// Create a new callback error
    pub fn callback_error(message: impl Into<String>) -> Self {
        EventRegistryError::CallbackError {
            message: message.into(),
            context: None,
        }
    }
    
    /// Create a new callback error with context
    pub fn callback_error_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        EventRegistryError::CallbackError {
            message: message.into(),
            context: Some(context.into()),
        }
    }
}

/// A type-erased event that can be stored in a registry
trait EventBase: Send + Sync + 'static {
    /// Get the type ID of this event
    fn type_id(&self) -> TypeId;
    
    /// Get the type name of this event for debugging
    fn type_name(&self) -> &'static str;
    
    /// Downcast this event to a concrete type
    fn as_any(&self) -> &dyn Any;
}

impl<T: Clone + Send + Sync + 'static> EventBase for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A trait for event publishers
pub trait EventPublisher<T: Clone + Send + Sync + 'static> {
    /// Publish an event to all subscribers
    async fn publish(&self, event: T) -> Result<usize, EventRegistryError>;
}

/// A trait for event subscribers
pub trait EventSubscriber<T: Clone + Send + Sync + 'static> {
    /// Subscribe to events with a callback
    async fn subscribe<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(T) -> Result<(), EventRegistryError> + Send + Sync + 'static;
        
    /// Unsubscribe from events
    async fn unsubscribe(&self, id: SubscriptionId) -> bool;
}

/// Identifier for event subscriptions
pub type SubscriptionId = Uuid;

/// A type-erased callback that can be stored in a registry
trait CallbackBase: Send + Sync {
    /// Call this callback with a type-erased event
    fn call(&self, event: &dyn EventBase) -> Result<(), EventRegistryError>;
    
    /// Get the expected event type for this callback
    fn expected_type_id(&self) -> TypeId;
    
    /// Get the expected type name for debugging
    fn expected_type_name(&self) -> &'static str;
}

/// A concrete callback for a specific event type
struct Callback<T: Clone + Send + Sync + 'static> {
    /// The function to call
    func: Box<dyn Fn(T) -> Result<(), EventRegistryError> + Send + Sync>,
    
    /// Phantom data to keep the type parameter
    _marker: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> Callback<T> {
    /// Create a new callback
    fn new<F>(func: F) -> Self
    where
        F: Fn(T) -> Result<(), EventRegistryError> + Send + Sync + 'static,
    {
        Self {
            func: Box::new(func),
            _marker: PhantomData,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> CallbackBase for Callback<T> {
    fn call(&self, event: &dyn EventBase) -> Result<(), EventRegistryError> {
        // Verify the event is of the expected type
        if event.type_id() != TypeId::of::<T>() {
            return Err(EventRegistryError::TypeMismatch {
                expected: std::any::type_name::<T>().to_string(),
                actual: event.type_name().to_string(),
            });
        }
        
        // Downcast the event to the expected type
        let event = match event.as_any().downcast_ref::<T>() {
            Some(event) => event,
            None => {
                // This should never happen if the type check above passed
                return Err(EventRegistryError::TypeMismatch {
                    expected: std::any::type_name::<T>().to_string(),
                    actual: event.type_name().to_string(),
                });
            }
        };
        
        // Call the callback with the event
        (self.func)(event.clone())
    }
    
    fn expected_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn expected_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// A registry for a specific event type
pub struct EventTypeRegistry<T: Clone + Send + Sync + 'static> {
    /// Callbacks registered with this registry
    callbacks: SharedState<HashMap<SubscriptionId, Arc<dyn CallbackBase>>>,
    
    /// Phantom data to keep the type parameter
    _marker: PhantomData<T>,
    
    /// Name for this event type (for logging)
    name: String,
}

impl<T: Clone + Send + Sync + 'static> EventTypeRegistry<T> {
    /// Create a new event type registry
    pub fn new() -> Self {
        Self::with_name(std::any::type_name::<T>())
    }
    
    /// Create a new event type registry with a custom name
    pub fn with_name(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!(event_type = %name, "Creating new EventTypeRegistry");
        
        Self {
            callbacks: SharedState::with_name(HashMap::new(), format!("event_registry_{}", name)),
            _marker: PhantomData,
            name,
        }
    }
    
    /// Get the number of subscribers
    pub async fn subscriber_count(&self) -> Result<usize, EventRegistryError> {
        let count = self.callbacks.read(|callbacks| callbacks.len()).await?;
        Ok(count)
    }
}

impl<T: Clone + Send + Sync + 'static> EventPublisher<T> for EventTypeRegistry<T> {
    async fn publish(&self, event: T) -> Result<usize, EventRegistryError> {
        let event_ref = &event as &dyn EventBase;
        
        // Get all callbacks
        let callbacks = self.callbacks.read(|callbacks| {
            callbacks.iter().map(|(id, callback)| (*id, Arc::clone(callback))).collect::<Vec<_>>()
        }).await?;
        
        trace!(
            event_type = %self.name,
            subscriber_count = callbacks.len(),
            "Publishing event to subscribers"
        );
        
        if callbacks.is_empty() {
            return Ok(0);
        }
        
        let mut success_count = 0;
        let mut errors = Vec::new();
        
        // Call each callback
        for (id, callback) in callbacks {
            match callback.call(event_ref) {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    warn!(
                        subscription_id = %id,
                        error = %e,
                        "Error in event callback"
                    );
                    errors.push((id, e));
                }
            }
        }
        
        // If all callbacks succeeded, return success
        if errors.is_empty() {
            trace!(
                event_type = %self.name,
                success_count,
                "All event callbacks succeeded"
            );
            Ok(success_count)
        } else {
            // Report the error but don't fail the entire publish operation
            warn!(
                event_type = %self.name,
                success_count,
                error_count = errors.len(),
                "Some event callbacks failed"
            );
            Ok(success_count)
        }
    }
}

impl<T: Clone + Send + Sync + 'static> EventSubscriber<T> for EventTypeRegistry<T> {
    async fn subscribe<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(T) -> Result<(), EventRegistryError> + Send + Sync + 'static,
    {
        let id = Uuid::new_v4();
        let callback = Arc::new(Callback::new(callback));
        
        // Register the callback
        match self.callbacks.write(|callbacks| {
            callbacks.insert(id, callback);
        }).await {
            Ok(()) => {
                debug!(
                    event_type = %self.name,
                    subscription_id = %id,
                    "Registered event callback"
                );
            },
            Err(e) => {
                error!(
                    event_type = %self.name,
                    error = %e,
                    "Failed to register event callback"
                );
            }
        }
        
        id
    }
    
    async fn unsubscribe(&self, id: SubscriptionId) -> bool {
        match self.callbacks.write(|callbacks| {
            callbacks.remove(&id).is_some()
        }).await {
            Ok(removed) => {
                if removed {
                    debug!(
                        event_type = %self.name,
                        subscription_id = %id,
                        "Unregistered event callback"
                    );
                } else {
                    warn!(
                        event_type = %self.name,
                        subscription_id = %id,
                        "Attempted to unregister non-existent event callback"
                    );
                }
                removed
            }
            Err(e) => {
                error!(
                    event_type = %self.name,
                    subscription_id = %id,
                    error = %e,
                    "Failed to unregister event callback"
                );
                false
            }
        }
    }
}

impl<T: Clone + Send + Sync + 'static> fmt::Debug for EventTypeRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventTypeRegistry")
            .field("event_type", &std::any::type_name::<T>())
            .field("name", &self.name)
            .finish()
    }
}

/// Central registry for all event types
#[derive(Debug, Clone)]
pub struct EventRegistry {
    /// Name for this registry (used in logging)
    name: String,
    
    /// Registries for different event types
    registries: SharedState<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl EventRegistry {
    /// Create a new event registry
    pub fn new() -> Self {
        Self::with_name("unnamed")
    }
    
    /// Create a new event registry with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!(name = %name, "Creating new EventRegistry");
        
        Self {
            name: name.clone(),
            registries: SharedState::with_name(HashMap::new(), format!("event_registry_{}", name)),
        }
    }
    
    /// Get or create a registry for a specific event type
    pub async fn registry<T: Clone + Send + Sync + 'static>(&self) -> Result<Arc<EventTypeRegistry<T>>, EventRegistryError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let result = self.registries.write(|registries| {
            // Check if registry exists
            if let Some(registry) = registries.get(&type_id) {
                // Try to downcast
                match registry.downcast_ref::<Arc<EventTypeRegistry<T>>>() {
                    Some(registry) => Arc::clone(registry),
                    None => {
                        // This should never happen - type IDs should be unique
                        warn!(
                            registry = %self.name,
                            event_type = %type_name,
                            "Type ID collision in event registry"
                        );
                        
                        // Create a new registry if downcast fails
                        let registry = Arc::new(EventTypeRegistry::<T>::with_name(type_name));
                        registries.insert(type_id, Arc::new(registry.clone()));
                        registry
                    }
                }
            } else {
                // Create a new registry
                debug!(
                    registry = %self.name,
                    event_type = %type_name,
                    "Creating new event type registry"
                );
                
                let registry = Arc::new(EventTypeRegistry::<T>::with_name(type_name));
                registries.insert(type_id, Arc::new(registry.clone()));
                registry
            }
        }).await;
        
        // Convert LockError to EventRegistryError
        result.map_err(|e| e.into())
    }
    
    /// Get registry statistics
    pub async fn stats(&self) -> Result<HashMap<String, usize>, EventRegistryError> {
        self.registries.read(|registries| {
            // Collect stats about each registry
            let mut stats = HashMap::new();
            
            for (type_id, _registry) in registries {
                let type_name = registries
                    .keys()
                    .find(|&k| k == type_id)
                    .map(|_| format!("{:?}", type_id))
                    .unwrap_or_else(|| "unknown".to_string());
                
                stats.insert(type_name, 0); // Placeholder
            }
            
            stats
        }).await.map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test event types
    #[derive(Debug, Clone)]
    struct TestEvent {
        id: u32,
        message: String,
    }
    
    #[derive(Debug, Clone)]
    struct OtherEvent {
        value: i32,
    }
    
    #[tokio::test]
    async fn test_event_type_registry() {
        let registry = EventTypeRegistry::<TestEvent>::new();
        
        // Initially no subscribers
        assert_eq!(registry.subscriber_count().await.unwrap(), 0);
        
        // Subscribe
        let id = registry.subscribe(|event| {
            assert_eq!(event.id, 42);
            assert_eq!(event.message, "hello");
            Ok(())
        }).await;
        
        // Now one subscriber
        assert_eq!(registry.subscriber_count().await.unwrap(), 1);
        
        // Publish
        let event = TestEvent {
            id: 42,
            message: "hello".to_string(),
        };
        
        let successful = registry.publish(event).await.unwrap();
        assert_eq!(successful, 1);
        
        // Unsubscribe
        assert!(registry.unsubscribe(id).await);
        
        // No more subscribers
        assert_eq!(registry.subscriber_count().await.unwrap(), 0);
    }
    
    #[tokio::test]
    async fn test_event_registry() {
        let registry = EventRegistry::new();
        
        // Get registries for different event types
        let test_registry = registry.registry::<TestEvent>().await.unwrap();
        let other_registry = registry.registry::<OtherEvent>().await.unwrap();
        
        // Subscribe to TestEvent
        let test_id = test_registry.subscribe(|event| {
            assert_eq!(event.id, 1);
            assert_eq!(event.message, "test");
            Ok(())
        }).await;
        
        // Subscribe to OtherEvent
        let other_id = other_registry.subscribe(|event| {
            assert_eq!(event.value, 42);
            Ok(())
        }).await;
        
        // Publish events to the appropriate registries
        let test_event = TestEvent {
            id: 1,
            message: "test".to_string(),
        };
        
        let other_event = OtherEvent {
            value: 42,
        };
        
        let test_successful = test_registry.publish(test_event).await.unwrap();
        assert_eq!(test_successful, 1);
        
        let other_successful = other_registry.publish(other_event).await.unwrap();
        assert_eq!(other_successful, 1);
        
        // Unsubscribe from both
        assert!(test_registry.unsubscribe(test_id).await);
        assert!(other_registry.unsubscribe(other_id).await);
    }
    
    #[tokio::test]
    async fn test_callback_errors() {
        let registry = EventTypeRegistry::<TestEvent>::new();
        
        // Subscribe with a callback that returns an error
        registry.subscribe(|_| {
            Err(EventRegistryError::callback_error("test error"))
        }).await;
        
        // Publish - should not fail the entire publish but return success count of 0
        let event = TestEvent {
            id: 1,
            message: "test".to_string(),
        };
        
        let successful = registry.publish(event).await.unwrap();
        assert_eq!(successful, 0);
    }
}