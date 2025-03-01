use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, trace};

/// Generic event stream for reactive event handling
pub struct EventStream<T: Clone + Send + 'static> {
    sender: broadcast::Sender<T>,
    buffer: Arc<RwLock<VecDeque<T>>>,
    buffer_size: usize,
    stats: Arc<RwLock<EventStreamStats>>,
}

/// Statistics for monitoring stream activity
#[derive(Debug, Clone, Default)]
pub struct EventStreamStats {
    pub events_published: u64,
    pub events_dropped: u64,
    pub type_counts: HashMap<String, u64>,
}

impl<T: Clone + Send + 'static> EventStream<T> {
    /// Create a new event stream with specified capacity
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        info!(capacity, buffer_size, "Creating new event stream");
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(buffer_size))),
            buffer_size,
            stats: Arc::new(RwLock::new(EventStreamStats::default())),
        }
    }

    /// Subscribe to the event stream
    pub fn subscribe(&self) -> Subscriber<T> {
        debug!("New subscriber registered to event stream");
        Subscriber {
            receiver: self.sender.subscribe(),
            buffer: Arc::clone(&self.buffer),
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: T) -> Result<usize, broadcast::error::SendError<T>> {
        // Try to send the event
        let result = self.sender.send(event.clone());

        match &result {
            Ok(receiver_count) => {
                // Store event in buffer regardless of subscribers
                self.buffer_event(event).await;
                
                // Update stats asynchronously to avoid blocking
                let stats = Arc::clone(&self.stats);
                tokio::spawn(async move {
                    let mut stats_guard = stats.write().await;
                    stats_guard.events_published += 1;
                    trace!("Event published to {} receivers", receiver_count);
                });
                
                debug!(receivers = receiver_count, "Event published successfully");
            }
            Err(e) => {
                // If it failed but there are no receivers, still buffer the event
                // but don't count it as an error
                if e.to_string().contains("no receivers") {
                    self.buffer_event(event).await;
                    
                    let stats = Arc::clone(&self.stats);
                    tokio::spawn(async move {
                        let mut stats_guard = stats.write().await;
                        stats_guard.events_dropped += 1;
                    });
                    
                    debug!("No receivers for event, message buffered");
                } else {
                    error!(error = %e, "Failed to publish event");
                }
            }
        };

        result
    }

    /// Store an event in the buffer for replay
    async fn buffer_event(&self, event: T) {
        let mut buffer = self.buffer.write().await;
        buffer.push_back(event);
        
        // Keep buffer size under control
        while buffer.len() > self.buffer_size {
            buffer.pop_front();
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> EventStreamStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics counters
    pub async fn reset_stats(&self) {
        *self.stats.write().await = EventStreamStats::default();
        debug!("Event stream statistics reset to defaults");
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.buffer_size
    }
}

/// Subscriber for receiving events from a stream
pub struct Subscriber<T: Clone + Send + 'static> {
    receiver: broadcast::Receiver<T>,
    buffer: Arc<RwLock<VecDeque<T>>>,
}

impl<T: Clone + Send + 'static> Subscriber<T> {
    /// Receive the next event
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Replay events from the buffer
    pub async fn replay_buffer(&mut self) -> Vec<T> {
        let buffer = self.buffer.read().await;
        buffer.iter().cloned().collect()
    }
}

impl<T: Clone + Send + 'static> Clone for EventStream<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            buffer: Arc::clone(&self.buffer),
            buffer_size: self.buffer_size,
            stats: Arc::clone(&self.stats),
        }
    }
}