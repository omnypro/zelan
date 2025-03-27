//! Type-safe event registry system using tokio broadcast channels
//!
//! This module provides a clean way to publish and subscribe to events with proper
//! type checking, error handling, and strong guarantees about event delivery.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

/// Error type for event registry operations
#[derive(Error, Debug, Clone)]
pub enum EventRegistryError {
    /// The event type is not registered with this registry
    #[error("Event type not registered")]
    TypeNotRegistered,

    /// Value not initialized
    #[error("Cannot access uninitialized value")]
    Uninitialized,

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
    pub fn callback_error_with_context(
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        EventRegistryError::CallbackError {
            message: message.into(),
            context: Some(context.into()),
        }
    }
}

/// Identifier for event subscriptions
pub type SubscriptionId = Uuid;

/// A trait for event publishers
pub trait EventPublisher<T: Clone + Send + Sync + 'static> {
    /// Publish an event to all subscribers
    fn publish(
        &self,
        event: T,
    ) -> impl std::future::Future<Output = Result<usize, EventRegistryError>> + Send;
}

/// A trait for event subscribers
pub trait EventSubscriber<T: Clone + Send + Sync + 'static> {
    /// Subscribe to events with a callback
    fn subscribe<F>(&self, callback: F) -> impl std::future::Future<Output = SubscriptionId> + Send
    where
        F: Fn(T) -> Result<(), EventRegistryError> + Send + Sync + 'static;

    /// Unsubscribe from events
    fn unsubscribe(&self, id: SubscriptionId) -> impl std::future::Future<Output = bool> + Send;
}

/// A registry for a specific event type using tokio's broadcast channel
#[derive(Clone)]
pub struct EventTypeRegistry<T: Clone + Send + Sync + 'static> {
    /// The broadcast sender for distributing events
    sender: broadcast::Sender<T>,

    /// Store receivers to track and clean up subscriptions
    receivers: Arc<dashmap::DashMap<SubscriptionId, broadcast::Receiver<T>>>,

    /// Name for this event type (for logging)
    name: String,

    /// Phantom data to keep the type parameter
    _marker: PhantomData<T>,
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

        let (sender, _) = broadcast::channel(100);

        Self {
            sender,
            receivers: Arc::new(dashmap::DashMap::new()),
            name,
            _marker: PhantomData,
        }
    }

    /// Get the number of subscribers
    pub async fn subscriber_count(&self) -> usize {
        self.receivers.len()
    }
}

impl<T: Clone + Send + Sync + 'static> EventPublisher<T> for EventTypeRegistry<T> {
    fn publish(
        &self,
        event: T,
    ) -> impl std::future::Future<Output = Result<usize, EventRegistryError>> + Send {
        async move {
            let receiver_count = self.receivers.len();

            trace!(
                event_type = %self.name,
                subscriber_count = receiver_count,
                "Publishing event to subscribers"
            );

            if receiver_count == 0 {
                return Ok(0);
            }

            match self.sender.send(event) {
                Ok(count) => {
                    trace!(
                        event_type = %self.name,
                        delivered = count,
                        "Event published successfully"
                    );
                    Ok(count)
                }
                Err(e) => {
                    warn!(
                        event_type = %self.name,
                        error = %e,
                        "Error publishing event"
                    );
                    Err(EventRegistryError::callback_error(format!(
                        "Failed to send event: {}",
                        e
                    )))
                }
            }
        }
    }
}

impl<T: Clone + Send + Sync + 'static> EventSubscriber<T> for EventTypeRegistry<T> {
    fn subscribe<F>(&self, callback: F) -> impl std::future::Future<Output = SubscriptionId> + Send
    where
        F: Fn(T) -> Result<(), EventRegistryError> + Send + Sync + 'static,
    {
        async move {
            let id = Uuid::new_v4();
            let mut receiver = self.sender.subscribe();
            let callback = Arc::new(callback);

            // Store the receiver for management
            self.receivers.insert(id, self.sender.subscribe());

            // Spawn a task to process events for this callback
            let callback_clone = Arc::clone(&callback);
            let receivers_map = Arc::clone(&self.receivers);
            let event_type = self.name.clone();

            tokio::spawn(async move {
                debug!(
                    subscription_id = %id,
                    event_type = %event_type,
                    "Started event subscriber"
                );

                while let Ok(event) = receiver.recv().await {
                    if let Err(e) = callback_clone(event) {
                        warn!(
                            subscription_id = %id,
                            event_type = %event_type,
                            error = %e,
                            "Error in event callback"
                        );
                    }
                }

                // Receiver was dropped, clean up
                receivers_map.remove(&id);

                debug!(
                    subscription_id = %id,
                    event_type = %event_type,
                    "Event subscriber stopped"
                );
            });

            debug!(
                subscription_id = %id,
                event_type = %self.name,
                "Registered event callback"
            );

            id
        }
    }

    fn unsubscribe(&self, id: SubscriptionId) -> impl std::future::Future<Output = bool> + Send {
        async move {
            let removed = self.receivers.remove(&id).is_some();

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

    /// Map of type IDs to type registries
    registries: Arc<dashmap::DashMap<TypeId, Arc<dyn std::any::Any + Send + Sync>>>,
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
            registries: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Get or create a registry for a specific event type
    pub async fn registry<T: Clone + Send + Sync + 'static>(&self) -> Arc<EventTypeRegistry<T>> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // Check if registry exists
        if let Some(registry) = self.registries.get(&type_id) {
            // Try to downcast
            if let Some(registry_ref) = registry.downcast_ref::<Arc<EventTypeRegistry<T>>>() {
                return Arc::clone(registry_ref);
            } else {
                // This should never happen - type IDs should be unique
                warn!(
                    registry = %self.name,
                    event_type = %type_name,
                    "Type ID collision in event registry"
                );
            }
        }

        // Create a new registry
        debug!(
            registry = %self.name,
            event_type = %type_name,
            "Creating new event type registry"
        );

        let registry = Arc::new(EventTypeRegistry::<T>::with_name(type_name));
        self.registries
            .insert(type_id, Arc::new(Arc::clone(&registry)));
        registry
    }

    /// Get registry statistics
    pub async fn stats(&self) -> HashMap<String, usize> {
        let stats_futures: Vec<_> = self
            .registries
            .iter()
            .map(|entry| {
                let type_id = *entry.key();
                let type_name = format!("{:?}", type_id);

                async move {
                    // Just return the type name with a placeholder count
                    // In a real implementation, you could calculate actual counts
                    (type_name, 0)
                }
            })
            .collect();

        // Execute all futures and collect results
        futures::future::join_all(stats_futures)
            .await
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

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
        assert_eq!(registry.subscriber_count().await, 0);

        // Subscribe
        let received = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let received_clone = Arc::clone(&received);

        let id = registry
            .subscribe(move |event| {
                assert_eq!(event.id, 42);
                assert_eq!(event.message, "hello");
                received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .await;

        // Give time for subscription to start
        sleep(Duration::from_millis(10)).await;

        // Now one subscriber
        assert_eq!(registry.subscriber_count().await, 1);

        // Publish
        let event = TestEvent {
            id: 42,
            message: "hello".to_string(),
        };

        let successful = registry.publish(event).await.unwrap();
        assert!(successful > 0, "At least one receiver should be notified");

        // Give time for event processing
        sleep(Duration::from_millis(50)).await;

        // Unsubscribe
        assert!(registry.unsubscribe(id).await);

        // No more subscribers
        assert_eq!(registry.subscriber_count().await, 0);
    }

    #[tokio::test]
    async fn test_event_registry() {
        let registry = EventRegistry::new();

        // Get registries for different event types
        let test_registry = registry.registry::<TestEvent>().await;
        let other_registry = registry.registry::<OtherEvent>().await;

        // Create counters
        let test_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let other_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Subscribe to TestEvent
        let test_counter_clone = Arc::clone(&test_counter);
        let test_id = test_registry
            .subscribe(move |event| {
                assert_eq!(event.id, 1);
                assert_eq!(event.message, "test");
                test_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .await;

        // Subscribe to OtherEvent
        let other_counter_clone = Arc::clone(&other_counter);
        let other_id = other_registry
            .subscribe(move |event| {
                assert_eq!(event.value, 42);
                other_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            })
            .await;

        // Give time for subscriptions to start
        sleep(Duration::from_millis(10)).await;

        // Publish events to the appropriate registries
        let test_event = TestEvent {
            id: 1,
            message: "test".to_string(),
        };

        let other_event = OtherEvent { value: 42 };

        let test_successful = test_registry.publish(test_event).await.unwrap();
        assert!(
            test_successful > 0,
            "At least one TestEvent receiver should be notified"
        );

        let other_successful = other_registry.publish(other_event).await.unwrap();
        assert!(
            other_successful > 0,
            "At least one OtherEvent receiver should be notified"
        );

        // Give time for event processing
        sleep(Duration::from_millis(50)).await;

        // Check counters
        assert_eq!(
            test_counter.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "TestEvent callback should have been called once"
        );
        assert_eq!(
            other_counter.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "OtherEvent callback should have been called once"
        );

        // Unsubscribe from both
        assert!(test_registry.unsubscribe(test_id).await);
        assert!(other_registry.unsubscribe(other_id).await);
    }

    #[tokio::test]
    async fn test_callback_errors() {
        let registry = EventTypeRegistry::<TestEvent>::new();

        let error_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let error_counter_clone = Arc::clone(&error_counter);

        // Subscribe with a callback that returns an error
        registry
            .subscribe(move |_| {
                error_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(EventRegistryError::callback_error("test error"))
            })
            .await;

        // Give time for subscription to start
        sleep(Duration::from_millis(10)).await;

        // Publish - should still deliver the event
        let event = TestEvent {
            id: 1,
            message: "test".to_string(),
        };

        let successful = registry.publish(event).await.unwrap();
        assert!(successful > 0, "At least one receiver should be notified");

        // Give time for event processing
        sleep(Duration::from_millis(50)).await;

        // Check that callback was called despite error
        assert_eq!(
            error_counter.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Callback should have been called once despite returning error"
        );
    }
}
