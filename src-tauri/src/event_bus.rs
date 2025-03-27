use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::error;
use tracing::{info, trace, warn};

use crate::flow::TraceContext;
use crate::ZelanResult;

/// Standardized event structure for all events flowing through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Source service that generated the event (e.g., "obs", "twitch")
    pub source: String,
    /// Type of event (e.g., "scene.changed", "chat.message")
    pub event_type: String,
    /// Arbitrary JSON payload with event details
    pub payload: serde_json::Value,
    /// Timestamp when the event was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Message format version for backward compatibility
    #[serde(default = "default_version")]
    pub version: u8,
    /// Unique event ID
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// Trace context for observability (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_context: Option<TraceContext>,
}

/// Default version for events
fn default_version() -> u8 {
    1
}

/// Generate a UUID for events
fn generate_uuid() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

impl StreamEvent {
    /// Create a new event without trace context
    pub fn new(source: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now(),
            version: default_version(),
            id: generate_uuid(),
            trace_context: None,
        }
    }

    /// Create a new event with trace context
    pub fn new_with_trace(
        source: &str,
        event_type: &str,
        payload: serde_json::Value,
        trace_context: TraceContext,
    ) -> Self {
        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now(),
            version: default_version(),
            id: generate_uuid(),
            trace_context: Some(trace_context),
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

    /// Get the trace context if available
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace_context.as_ref()
    }

    /// Get a mutable reference to the trace context
    pub fn trace_context_mut(&mut self) -> Option<&mut TraceContext> {
        self.trace_context.as_mut()
    }

    /// Add trace context to an event
    pub fn with_trace_context(mut self, trace_context: TraceContext) -> Self {
        self.trace_context = Some(trace_context);
        self
    }
}

/// Statistics about event bus activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    /// Number of events successfully published
    pub events_published: u64,
    /// Number of events dropped (no receivers)
    pub events_dropped: u64,
    /// Count of events by source
    pub source_counts: HashMap<String, u64>,
    /// Count of events by type
    pub type_counts: HashMap<String, u64>,
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
    /// The broadcast channel sender
    sender: broadcast::Sender<StreamEvent>,
    /// Configured capacity of the channel
    capacity: usize,
    /// Statistics about event bus activity
    pub(crate) stats: Arc<RwLock<EventBusStats>>,
}

impl EventBus {
    /// Create a new event bus with the specified capacity
    pub fn new(capacity: usize) -> Self {
        info!(capacity, "Creating new event bus");
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            capacity,
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Get a receiver to subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        trace!("New subscriber registered to event bus");
        self.sender.subscribe()
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, mut event: StreamEvent) -> ZelanResult<usize> {
        // Cache details for statistics
        let source = event.source.clone();
        let event_type = event.event_type.clone();

        trace!(
            source = %source,
            event_type = %event_type,
            "Publishing event to bus"
        );

        // If the event has a trace context, add a span for the EventBus
        if let Some(trace_context) = event.trace_context_mut() {
            trace_context
                .add_span("publish", "EventBus")
                .context(Some(serde_json::json!({
                    "bus_capacity": self.capacity,
                    "subscribers": self.subscriber_count()
                })));
        }

        // Attempt to send the event
        match self.sender.send(event) {
            Ok(receivers) => {
                // Update statistics immediately instead of asynchronously to fix test issues
                let mut stats_guard = self.stats.write().await;
                stats_guard.events_published += 1;
                *stats_guard.source_counts.entry(source).or_insert(0) += 1;
                *stats_guard.type_counts.entry(event_type).or_insert(0) += 1;

                trace!(receivers, "Event published successfully");
                Ok(receivers)
            }
            Err(err) => {
                // Check if error is just "no receivers"
                if err.to_string().contains("no receivers") {
                    // Update dropped count immediately instead of asynchronously
                    let mut stats_guard = self.stats.write().await;
                    stats_guard.events_dropped += 1;

                    warn!(
                        source = %source,
                        event_type = %event_type,
                        "No receivers for event, message dropped"
                    );
                    Ok(0)
                } else {
                    error!(error = %err, "Failed to publish event");
                    Err(crate::error::event_bus_publish_failed(err))
                }
            }
        }
    }

    /// Get current event bus statistics
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
    pub async fn reset_stats(&self) {
        info!("Resetting event bus statistics");
        *self.stats.write().await = EventBusStats::default();
    }

    /// Get the configured capacity of the event bus
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            capacity: self.capacity,
            stats: Arc::clone(&self.stats),
        }
    }
}
