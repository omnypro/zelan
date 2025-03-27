//! Callback system for OBS events
//!
//! This module provides a structured way to handle OBS event callbacks using the centralized
//! callback system.

use anyhow::Result;
use serde_json::Value;
use tracing::{debug, error};

use crate::callback_system::CallbackRegistry;

/// Enum for different types of OBS events that can trigger callbacks
#[derive(Clone, Debug)]
pub enum ObsEvent {
    /// Scene changed in OBS
    SceneChanged {
        /// Name of the new scene
        scene_name: String,
        /// Additional scene data
        data: Value,
    },
    /// OBS connection state changed
    ConnectionChanged {
        /// Whether OBS is connected
        connected: bool,
        /// Any additional connection data
        data: Value,
    },
    /// OBS streaming state changed
    StreamStateChanged {
        /// Whether streaming is active
        active: bool,
        /// Any additional stream data
        data: Value,
    },
    /// OBS recording state changed
    RecordingStateChanged {
        /// Whether recording is active
        active: bool,
        /// Any additional recording data
        data: Value,
    },
    /// Generic OBS event for any other event types
    Generic {
        /// The specific type/name of the event
        event_type: String,
        /// The full event data
        data: Value,
    },
}

/// Helper method to get a string identifying the event type
impl ObsEvent {
    /// Get a string representation of the event type
    pub fn event_type(&self) -> &'static str {
        match self {
            ObsEvent::SceneChanged { .. } => "scene_changed",
            ObsEvent::ConnectionChanged { .. } => "connection_changed",
            ObsEvent::StreamStateChanged { .. } => "stream_state_changed",
            ObsEvent::RecordingStateChanged { .. } => "recording_state_changed",
            ObsEvent::Generic { .. } => "generic_event",
        }
    }
}

/// Dedicated registry for OBS event callbacks
#[derive(Clone)]
pub struct ObsCallbackRegistry {
    /// The callback registry for OBS events
    registry: CallbackRegistry<ObsEvent>,
}

impl ObsCallbackRegistry {
    /// Create a new OBS callback registry
    pub fn new() -> Self {
        Self {
            registry: CallbackRegistry::with_group("obs_events"),
        }
    }

    /// Register an OBS event callback function
    pub async fn register<F>(&self, callback: F) -> crate::callback_system::CallbackId
    where
        F: Fn(ObsEvent) -> Result<()> + Send + Sync + 'static,
    {
        let id = self.registry.register(callback).await;
        debug!(callback_id = %id, "Registered OBS event callback");
        id
    }

    /// Trigger all registered callbacks with the provided OBS event
    pub async fn trigger(&self, event: ObsEvent) -> Result<usize> {
        debug!(
            event_type = %event.event_type(),
            "Triggering OBS event callbacks"
        );

        match self.registry.trigger(event.clone()).await {
            Ok(count) => {
                debug!(
                    event_type = %event.event_type(),
                    callbacks_executed = count,
                    "Successfully triggered OBS event callbacks"
                );
                Ok(count)
            }
            Err(e) => {
                error!(
                    event_type = %event.event_type(),
                    error = %e,
                    "Error triggering OBS event callbacks"
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
