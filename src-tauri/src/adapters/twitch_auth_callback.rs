//! Improved callback system for Twitch authentication
//! 
//! This module provides a better way to handle auth callbacks using the centralized
//! callback system.

use anyhow::Result;
use tracing::{debug, error};

use crate::callback_system::CallbackRegistry;
use super::twitch_auth::AuthEvent;

/// Dedicated registry for auth callbacks with improved callback management
#[derive(Clone)]
pub struct TwitchAuthCallbackRegistry {
    /// The callback registry for auth events
    registry: CallbackRegistry<AuthEvent>,
}

impl TwitchAuthCallbackRegistry {
    /// Create a new auth callback registry
    pub fn new() -> Self {
        Self {
            registry: CallbackRegistry::with_group("twitch_auth"),
        }
    }
    
    /// Register an auth callback function
    pub async fn register<F>(&self, callback: F) -> crate::callback_system::CallbackId
    where
        F: Fn(AuthEvent) -> Result<()> + Send + Sync + 'static,
    {
        let id = self.registry.register(callback).await;
        debug!(callback_id = %id, "Registered Twitch auth callback");
        id
    }
    
    /// Trigger all registered callbacks with the provided auth event
    pub async fn trigger(&self, event: AuthEvent) -> Result<usize> {
        debug!(
            event_type = %event.event_type(),
            "Triggering Twitch auth callbacks"
        );
        
        match self.registry.trigger(event.clone()).await {
            Ok(count) => {
                debug!(
                    event_type = %event.event_type(),
                    callbacks_executed = count,
                    "Successfully triggered Twitch auth callbacks"
                );
                Ok(count)
            },
            Err(e) => {
                error!(
                    event_type = %event.event_type(),
                    error = %e,
                    "Error triggering Twitch auth callbacks"
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