//! Callback system for Test adapter events
//! 
//! This module provides a structured way to handle test events using the centralized
//! callback system.

use anyhow::Result;
use serde_json::Value;
use tracing::{debug, error};

use crate::callback_system::CallbackRegistry;

/// Enum for different types of test events that can trigger callbacks
#[derive(Clone, Debug)]
pub enum TestEvent {
    /// Regular test event
    Standard {
        /// Counter value
        counter: u64,
        /// Event message
        message: String,
        /// Additional event data
        data: Value,
    },
    /// Special test event (generated every 5 counts)
    Special {
        /// Counter value  
        counter: u64,
        /// Event message
        message: String,
        /// Additional event data
        data: Value,
    },
    /// Initial test event (generated at startup)
    Initial {
        /// Counter value
        counter: u64,
        /// Event message
        message: String,
        /// Additional event data
        data: Value,
    },
}

impl TestEvent {
    /// Get a string representation of the event type
    pub fn event_type(&self) -> &'static str {
        match self {
            TestEvent::Standard { .. } => "standard",
            TestEvent::Special { .. } => "special",
            TestEvent::Initial { .. } => "initial",
        }
    }
}

/// Dedicated registry for test event callbacks
#[derive(Clone)]
pub struct TestCallbackRegistry {
    /// The callback registry for test events
    registry: CallbackRegistry<TestEvent>,
}

impl TestCallbackRegistry {
    /// Create a new test event callback registry
    pub fn new() -> Self {
        Self {
            registry: CallbackRegistry::with_group("test_events"),
        }
    }
    
    /// Register a test event callback function
    pub async fn register<F>(&self, callback: F) -> crate::callback_system::CallbackId
    where
        F: Fn(TestEvent) -> Result<()> + Send + Sync + 'static,
    {
        let id = self.registry.register(callback).await;
        debug!(callback_id = %id, "Registered test event callback");
        id
    }
    
    /// Trigger all registered callbacks with the provided test event
    pub async fn trigger(&self, event: TestEvent) -> Result<usize> {
        debug!(
            event_type = %event.event_type(),
            "Triggering test event callbacks"
        );
        
        match self.registry.trigger(event.clone()).await {
            Ok(count) => {
                debug!(
                    event_type = %event.event_type(),
                    callbacks_executed = count,
                    "Successfully triggered test event callbacks"
                );
                Ok(count)
            },
            Err(e) => {
                error!(
                    event_type = %event.event_type(),
                    error = %e,
                    "Error triggering test event callbacks"
                );
                Err(e)
            }
        }
    }
    
    /// Get the number of registered callbacks
    pub async fn count(&self) -> usize {
        self.registry.count().await
    }
}