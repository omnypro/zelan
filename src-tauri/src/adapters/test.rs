// src/adapters/test.rs
use crate::{ServiceAdapter, EventBus, StreamEvent};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};

/// A simple test adapter that generates events at regular intervals
/// Useful for testing the event bus and WebSocket server without external services
pub struct TestAdapter {
    name: String,
    event_bus: Arc<EventBus>,
    connected: AtomicBool,
    task_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl TestAdapter {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            name: "test".to_string(),
            event_bus,
            connected: AtomicBool::new(false),
            task_handle: tokio::sync::Mutex::new(None),
        }
    }
    
    /// Generate test events at a regular interval
    async fn generate_events(&self) -> Result<()> {
        let mut counter = 0;
        
        while self.connected.load(Ordering::SeqCst) {
            // Generate a test event
            let event = StreamEvent::new(
                "test",
                "test.event",
                json!({
                    "counter": counter,
                    "message": format!("Test event #{}", counter),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            );
            
            // Publish the event
            if let Err(e) = self.event_bus.publish(event).await {
                eprintln!("Failed to publish test event: {}", e);
            }
            
            // Generate a different event every 5 counts
            if counter % 5 == 0 {
                let event = StreamEvent::new(
                    "test",
                    "test.special",
                    json!({
                        "counter": counter,
                        "special": true,
                        "message": "This is a special test event",
                    })
                );
                
                if let Err(e) = self.event_bus.publish(event).await {
                    eprintln!("Failed to publish special test event: {}", e);
                }
            }
            
            counter += 1;
            sleep(Duration::from_secs(1)).await;
        }
        
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for TestAdapter {
    async fn connect(&self) -> Result<()> {
        // Only connect if not already connected
        if self.connected.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Set connected state
        self.connected.store(true, Ordering::SeqCst);
        
        // Start generating events in a background task
        let self_clone = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = self_clone.generate_events().await {
                eprintln!("Error in test event generator: {}", e);
            }
        });
        
        // Store the task handle
        *self.task_handle.lock().await = Some(handle);
        
        println!("Test adapter connected and generating events");
        Ok(())
    }
    
    async fn disconnect(&self) -> Result<()> {
        // Set disconnected state to stop event generation
        self.connected.store(false, Ordering::SeqCst);
        
        // Wait for the task to complete
        if let Some(handle) = self.task_handle.lock().await.take() {
            // We don't want to await the handle here as it might be waiting
            // for the connected flag to change, which we just did above
            handle.abort();
        }
        
        println!("Test adapter disconnected");
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    
    fn get_name(&self) -> &str {
        &self.name
    }
    
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        println!("Test adapter configured with: {}", config);
        Ok(())
    }
}

impl Clone for TestAdapter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            event_bus: Arc::clone(&self.event_bus),
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
            task_handle: tokio::sync::Mutex::new(None),
        }
    }
}