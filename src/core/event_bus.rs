//! Event bus implementation
//! 
//! This module provides a central EventBus system for distributing events 
//! from various adapters to subscribers throughout the application.

use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, instrument, trace};
use uuid::Uuid;

use crate::error::{self, ZelanResult};

/// Default version for events
fn default_version() -> u8 {
    1
}

/// Generate a UUID for events
fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Standardized event structure for all events flowing through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Source service that generated the event (e.g., "obs", "twitch")
    source: String,
    /// Type of event (e.g., "scene.changed", "chat.message")
    event_type: String,
    /// Arbitrary JSON payload with event details
    payload: serde_json::Value,
    /// Timestamp when the event was created
    timestamp: chrono::DateTime<chrono::Utc>,
    /// Message format version for backward compatibility
    #[serde(default = "default_version")]
    version: u8,
    /// Unique event ID
    #[serde(default = "generate_uuid")]
    id: String,
}

impl StreamEvent {
    pub fn new(source: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: Utc::now(),
            version: default_version(),
            id: generate_uuid(),
        }
    }

    /// Get the source of this event
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Get the payload
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}

/// Statistics about event bus activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    events_published: u64,
    events_dropped: u64,
    source_counts: HashMap<String, u64>,
    type_counts: HashMap<String, u64>,
}

impl Default for EventBusStats {
    fn default() -> Self {
        Self {
            events_published: 0,
            events_dropped: 0,
            source_counts: HashMap::new(),
            type_counts: HashMap::new(),
        }
    }
}

/// Central event bus for distributing events from adapters to subscribers
pub struct EventBus {
    sender: broadcast::Sender<StreamEvent>,
    buffer_size: usize,
    stats: Arc<RwLock<EventBusStats>>,
}

impl EventBus {
    #[instrument(level = "debug")]
    pub fn new(capacity: usize) -> Self {
        info!(capacity, "Creating new event bus");
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            buffer_size: capacity,
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Get a subscriber to receive events
    #[instrument(skip(self), level = "debug")]
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        debug!("New subscriber registered to event bus");
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers with optimized statistics handling
    #[instrument(skip(self, event), fields(source = %event.source(), event_type = %event.event_type()), level = "debug")]
    pub async fn publish(&self, event: StreamEvent) -> ZelanResult<usize> {
        // Cache event details before acquiring lock
        let source = event.source.clone();
        let event_type = event.event_type.clone();

        debug!("Publishing event to bus");

        // Attempt to send the event first (most common operation)
        match self.sender.send(event) {
            Ok(receivers) => {
                // Only update stats after successful send, using a separate task
                let stats = Arc::clone(&self.stats);
                tokio::spawn(async move {
                    let mut stats_guard = stats.write().await;
                    stats_guard.events_published += 1;
                    *stats_guard.source_counts.entry(source).or_insert(0) += 1;
                    *stats_guard.type_counts.entry(event_type).or_insert(0) += 1;
                });

                debug!(receivers, "Event published successfully");
                Ok(receivers)
            }
            Err(err) => {
                // If error indicates no subscribers, just record the statistic but don't treat as error
                if err.to_string().contains("channel closed") || err.to_string().contains("no receivers") {
                    // Update dropped count in a separate task
                    let stats = Arc::clone(&self.stats);
                    tokio::spawn(async move {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_dropped += 1;
                    });

                    debug!(
                        source = %source,
                        event_type = %event_type,
                        "No receivers for event, message dropped"
                    );
                    Ok(0) // Return 0 receivers instead of an error
                } else {
                    // Update dropped count for other errors
                    let stats = Arc::clone(&self.stats);
                    tokio::spawn(async move {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_dropped += 1;
                    });

                    error!(error = %err, "Failed to publish event");
                    Err(error::event_bus_publish_failed(err))
                }
            }
        }
    }

    /// Get current event bus statistics
    #[instrument(skip(self), level = "trace")]
    pub async fn get_stats(&self) -> EventBusStats {
        let stats = self.stats.read().await.clone();
        trace!(
            events_published = stats.events_published,
            events_dropped = stats.events_dropped,
            "Retrieved event bus statistics"
        );
        stats
    }

    /// Reset all statistics counters
    #[instrument(skip(self), level = "debug")]
    pub async fn reset_stats(&self) {
        info!("Resetting event bus statistics");
        *self.stats.write().await = EventBusStats::default();
        debug!("Event bus statistics reset to defaults");
    }

    /// Get the configured capacity of the event bus
    #[instrument(skip(self), level = "debug")]
    pub fn capacity(&self) -> usize {
        debug!(capacity = self.buffer_size, "Retrieved event bus capacity");
        self.buffer_size
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            buffer_size: self.buffer_size,
            stats: Arc::clone(&self.stats),
        }
    }
}