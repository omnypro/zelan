use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod streams;
pub use streams::{EventStream, EventStreamStats, Subscriber};

// Event bus capacity constant
pub const EVENT_BUS_CAPACITY: usize = 1000;
pub const EVENT_BUFFER_SIZE: usize = 100;

/// Standardized event structure for all events flowing through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Source service that generated the event (e.g., "obs", "twitch")
    source: String,
    /// Type of event (e.g., "scene.changed", "chat.message")
    event_type: String,
    /// Arbitrary JSON payload with event details
    payload: Value,
    /// Timestamp when the event was created
    timestamp: DateTime<Utc>,
}

impl StreamEvent {
    pub fn new(source: &str, event_type: &str, payload: Value) -> Self {
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: Utc::now(),
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
    pub fn payload(&self) -> &Value {
        &self.payload
    }
    
    /// Get the timestamp
    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
}

/// Central event bus using reactive streams
#[derive(Clone)]
pub struct EventBus {
    stream: Arc<EventStream<StreamEvent>>,
}

impl EventBus {
    /// Create a new event bus with the specified capacity
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        Self {
            stream: Arc::new(EventStream::new(capacity, buffer_size)),
        }
    }
    
    /// Get a subscriber to receive events
    pub fn subscribe(&self) -> Subscriber<StreamEvent> {
        self.stream.subscribe()
    }
    
    /// Publish an event to all subscribers
    pub async fn publish(&self, event: StreamEvent) -> Result<usize, tokio::sync::broadcast::error::SendError<StreamEvent>> {
        self.stream.publish(event).await
    }
    
    /// Create and publish a new event
    pub async fn publish_new(&self, source: &str, event_type: &str, payload: Value) -> Result<usize, tokio::sync::broadcast::error::SendError<StreamEvent>> {
        let event = StreamEvent::new(source, event_type, payload);
        self.publish(event).await
    }
    
    /// Get current statistics
    pub async fn get_stats(&self) -> EventStreamStats {
        self.stream.get_stats().await
    }
    
    /// Reset statistics
    pub async fn reset_stats(&self) {
        self.stream.reset_stats().await
    }
    
    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.stream.capacity()
    }
}