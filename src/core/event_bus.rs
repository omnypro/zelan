//! Event bus implementation

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
    version: u8,
    /// Unique event ID
    id: String,
}

/// Central event bus for distributing events from adapters to subscribers
pub struct EventBus {
    // Placeholder implementation
}

/// Statistics about event bus activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    events_published: u64,
    events_dropped: u64,
    source_counts: HashMap<String, u64>,
    type_counts: HashMap<String, u64>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(_capacity: usize) -> Self {
        Self {}
    }
}