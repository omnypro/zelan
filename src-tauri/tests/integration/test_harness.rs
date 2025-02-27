//! Integration test harness for Zelan
//! Provides utilities for testing the EventBus and adapters

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio::time::sleep;

use zelan_lib::{EventBus, EventBusStats, ServiceAdapter, StreamEvent};
use zelan_lib::adapters::test::{TestAdapter, TestConfig};

/// Subscriber that collects events for testing
pub struct TestSubscriber {
    /// Name of the subscriber
    name: String,
    /// Collected events
    events: Mutex<Vec<StreamEvent>>,
    /// Event receiver
    _receiver: Receiver<StreamEvent>,
}

impl TestSubscriber {
    /// Create a new test subscriber
    pub fn new(name: &str, receiver: Receiver<StreamEvent>) -> Self {
        Self {
            name: name.to_string(),
            events: Mutex::new(Vec::new()),
            _receiver: receiver,
        }
    }

    /// Get all collected events
    pub async fn get_events(&self) -> Vec<StreamEvent> {
        self.events.lock().await.clone()
    }

    /// Clear collected events
    pub async fn clear_events(&self) {
        self.events.lock().await.clear();
    }

    /// Start collecting events in the background
    pub async fn start_collecting(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let self_clone = Arc::clone(self);
        let mut receiver = self._receiver.resubscribe();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let mut events = self_clone.events.lock().await;
                        events.push(event);
                    }
                    Err(e) => {
                        // Channel closed or other error
                        println!("Subscriber {}: Channel error: {}", self_clone.name, e);
                        break;
                    }
                }
            }
        })
    }
}

/// Test environment for integration tests
pub struct TestEnvironment {
    /// Event bus
    pub event_bus: Arc<EventBus>,
    /// Test adapter
    pub test_adapter: Arc<TestAdapter>,
    /// Test subscribers
    pub subscribers: HashMap<String, Arc<TestSubscriber>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Self {
        // Create event bus with a reasonable capacity
        let event_bus = Arc::new(EventBus::new(100));
        
        // Create test adapter with custom config
        let config = TestConfig {
            interval_ms: 100, // Faster events for testing
            generate_special_events: true,
        };
        
        let test_adapter = Arc::new(TestAdapter::with_config(event_bus.clone(), config));
        
        Self {
            event_bus,
            test_adapter,
            subscribers: HashMap::new(),
        }
    }

    /// Add a subscriber
    pub async fn add_subscriber(&mut self, name: &str) -> Arc<TestSubscriber> {
        let receiver = self.event_bus.subscribe();
        let subscriber = Arc::new(TestSubscriber::new(name, receiver));
        
        // Start collecting events
        subscriber.start_collecting().await;
        
        self.subscribers.insert(name.to_string(), Arc::clone(&subscriber));
        subscriber
    }
    
    /// Start the test adapter
    pub async fn start_adapter(&self) -> Result<()> {
        self.test_adapter.connect().await
    }
    
    /// Stop the test adapter
    pub async fn stop_adapter(&self) -> Result<()> {
        self.test_adapter.disconnect().await
    }
    
    /// Wait for a specific number of events to be collected
    pub async fn wait_for_events(&self, subscriber_name: &str, count: usize, timeout_ms: u64) -> Result<bool> {
        let subscriber = self.subscribers.get(subscriber_name).ok_or_else(|| {
            anyhow::anyhow!("Subscriber not found: {}", subscriber_name)
        })?;
        
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        while start.elapsed() < timeout {
            let events = subscriber.get_events().await;
            if events.len() >= count {
                return Ok(true);
            }
            sleep(Duration::from_millis(10)).await;
        }
        
        Ok(false)
    }
    
    /// Get events collected by a subscriber
    pub async fn get_subscriber_events(&self, subscriber_name: &str) -> Result<Vec<StreamEvent>> {
        let subscriber = self.subscribers.get(subscriber_name).ok_or_else(|| {
            anyhow::anyhow!("Subscriber not found: {}", subscriber_name)
        })?;
        
        Ok(subscriber.get_events().await)
    }
    
    /// Get event bus statistics
    pub async fn get_stats(&self) -> EventBusStats {
        self.event_bus.get_stats().await
    }
    
    /// Get the number of events published
    pub async fn get_published_count(&self) -> u64 {
        // Convert the stats to JSON to access the private fields
        let stats = self.event_bus.get_stats().await;
        let json = serde_json::to_value(&stats).unwrap();
        json["events_published"].as_u64().unwrap_or(0)
    }
    
    /// Get the number of events dropped
    pub async fn get_dropped_count(&self) -> u64 {
        // Convert the stats to JSON to access the private fields
        let stats = self.event_bus.get_stats().await;
        let json = serde_json::to_value(&stats).unwrap();
        json["events_dropped"].as_u64().unwrap_or(0)
    }
    
    /// Reset event bus statistics
    pub async fn reset_stats(&self) {
        self.event_bus.reset_stats().await;
    }
}