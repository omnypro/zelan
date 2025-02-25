// Base adapter implementation with common functionality
use crate::{EventBus, StreamEvent};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, instrument, warn};

// Constants shared across adapters
pub const SHUTDOWN_CHANNEL_SIZE: usize = 1;

/// Base trait for adapter configuration
pub trait AdapterConfig: Clone + Send + Sync + std::fmt::Debug {
    /// Convert the configuration to a JSON value for storage
    fn to_json(&self) -> serde_json::Value;
    
    /// Create a configuration from a JSON value
    fn from_json(json: &serde_json::Value) -> Result<Self> where Self: Sized;
    
    /// Get a string identifying the adapter type
    fn adapter_type() -> &'static str where Self: Sized;
    
    /// Validate the configuration
    fn validate(&self) -> Result<()>;
}

/// Base adapter implementation with common functionality
pub struct BaseAdapter {
    /// Name of the adapter
    name: String,
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Whether the adapter is currently connected
    connected: AtomicBool,
    /// Shutdown signal channel
    shutdown_signal: Mutex<Option<mpsc::Sender<()>>>,
    /// Event handler task
    event_handler: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Clone for BaseAdapter {
    fn clone(&self) -> Self {
        // Create a new instance with the same name and event bus
        Self {
            name: self.name.clone(),
            event_bus: Arc::clone(&self.event_bus),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            shutdown_signal: Mutex::new(None),  // Don't clone the shutdown channel
            event_handler: Mutex::new(None),    // Don't clone the task handle
        }
    }
}

impl BaseAdapter {
    /// Create a new base adapter
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(name: &str, event_bus: Arc<EventBus>) -> Self {
        info!(adapter = %name, "Creating new adapter");
        Self {
            name: name.to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            shutdown_signal: Mutex::new(None),
            event_handler: Mutex::new(None),
        }
    }

    /// Get the name of the adapter
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a reference to the event bus
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Check if the adapter is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Set the connected state
    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::SeqCst);
    }
    
    /// Store the event handler task
    #[instrument(skip(self, handle), level = "debug")]
    pub async fn set_event_handler(&self, handle: tokio::task::JoinHandle<()>) {
        debug!(adapter = %self.name, "Storing event handler task");
        *self.event_handler.lock().await = Some(handle);
    }
    
    /// Store the shutdown signal sender
    #[instrument(skip(self, sender), level = "debug")]
    pub async fn set_shutdown_signal(&self, sender: mpsc::Sender<()>) {
        debug!(adapter = %self.name, "Storing shutdown signal sender");
        *self.shutdown_signal.lock().await = Some(sender);
    }
    
    /// Create a shutdown channel
    #[instrument(skip(self), level = "debug")]
    pub async fn create_shutdown_channel(&self) -> (mpsc::Sender<()>, mpsc::Receiver<()>) {
        debug!(adapter = %self.name, "Creating shutdown channel");
        let (shutdown_tx, shutdown_rx) = mpsc::channel(SHUTDOWN_CHANNEL_SIZE);
        self.set_shutdown_signal(shutdown_tx.clone()).await;
        (shutdown_tx, shutdown_rx)
    }
    
    /// Send shutdown signal and stop event handler
    #[instrument(skip(self), level = "debug")]
    pub async fn stop_event_handler(&self) -> Result<()> {
        if let Some(shutdown_sender) = self.shutdown_signal.lock().await.take() {
            debug!(adapter = %self.name, "Sending shutdown signal");
            let _ = shutdown_sender.send(()).await;
            // Small delay to allow shutdown signal to be processed
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        } else {
            debug!(adapter = %self.name, "No shutdown signal channel available");
        }

        // Cancel the event handler task if it's still running
        if let Some(handle) = self.event_handler.lock().await.take() {
            debug!(adapter = %self.name, "Aborting event handler task");
            handle.abort();
        } else {
            debug!(adapter = %self.name, "No event handler task to abort");
        }
        
        Ok(())
    }
    
    /// Publish an event to the event bus
    #[instrument(skip(self, payload), level = "debug")]
    pub async fn publish_event(&self, event_type: &str, payload: serde_json::Value) -> Result<usize> {
        debug!(
            adapter = %self.name,
            event_type = %event_type,
            "Publishing event"
        );
        
        let stream_event = StreamEvent::new(&self.name, event_type, payload);
        match self.event_bus.publish(stream_event).await {
            Ok(receivers) => {
                debug!(
                    adapter = %self.name,
                    event_type = %event_type,
                    receivers = receivers,
                    "Event published successfully"
                );
                Ok(receivers)
            }
            Err(e) => {
                error!(
                    adapter = %self.name,
                    event_type = %event_type,
                    error = %e,
                    "Failed to publish event"
                );
                Err(e.into())
            }
        }
    }
}