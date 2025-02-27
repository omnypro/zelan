//! Integration tests for adapter lifecycle
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde_json::json;

use zelan_lib::{EventBus, ServiceAdapter};
use zelan_lib::adapters::test::{TestAdapter, TestConfig};

/// Test that the adapter can start and stop properly
#[tokio::test]
async fn test_adapter_lifecycle() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create test adapter
    let adapter = TestAdapter::new(event_bus.clone());
    
    // Check initial state
    assert!(!adapter.is_connected(), "Adapter should start disconnected");
    
    // Connect the adapter
    adapter.connect().await?;
    
    // Should now be connected
    assert!(adapter.is_connected(), "Adapter should be connected after connect()");
    
    // Give it a bit of time to start event generation
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Check that events are being generated
    let stats_json = serde_json::to_value(&event_bus.get_stats().await).unwrap();
    let published = stats_json["events_published"].as_u64().unwrap_or(0);
    assert!(published > 0, "Adapter should generate events");
    
    // Disconnect the adapter
    adapter.disconnect().await?;
    
    // Should now be disconnected
    assert!(!adapter.is_connected(), "Adapter should be disconnected after disconnect()");
    
    // Record the event count
    let stats_json = serde_json::to_value(&event_bus.get_stats().await).unwrap();
    let events_before = stats_json["events_published"].as_u64().unwrap_or(0);
    
    // Wait a bit to ensure no more events are generated
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Event count should not increase
    let stats_json = serde_json::to_value(&event_bus.get_stats().await).unwrap();
    let events_after = stats_json["events_published"].as_u64().unwrap_or(0);
    assert_eq!(
        events_before, 
        events_after, 
        "No more events should be generated after disconnect"
    );
    
    Ok(())
}

/// Test reconfiguring an adapter during operation
#[tokio::test]
async fn test_adapter_reconfiguration() -> Result<()> {
    // Create event bus and subscriber
    let event_bus = Arc::new(EventBus::new(100));
    let receiver = event_bus.subscribe();
    let subscriber = Arc::new(crate::test_harness::TestSubscriber::new("config_test", receiver));
    subscriber.start_collecting().await;
    
    // Create test adapter with slow event rate
    let config = TestConfig {
        interval_ms: 500, // Slow - 2 events per second
        generate_special_events: false,
    };
    let adapter = Arc::new(TestAdapter::with_config(event_bus.clone(), config));
    
    // Connect the adapter
    adapter.connect().await?;
    
    // Wait for a couple of events
    tokio::time::sleep(Duration::from_millis(1100)).await;
    
    // Should have ~2 events
    let event_count_slow = subscriber.get_events().await.len();
    assert!(event_count_slow >= 1 && event_count_slow <= 3, 
        "Should have 1-3 events with slow rate, got {}", event_count_slow);
    
    // Clear events
    subscriber.clear_events().await;
    
    // Reconfigure for faster events
    let fast_config = json!({
        "interval_ms": 100, // Faster - 10 events per second
        "generate_special_events": true,
    });
    adapter.configure(fast_config).await?;
    
    // Wait the same amount of time
    tokio::time::sleep(Duration::from_millis(1100)).await;
    
    // Should have more events now
    let event_count_fast = subscriber.get_events().await.len();
    assert!(event_count_fast >= 8, 
        "Should have 8+ events with fast rate, got {}", event_count_fast);
    
    // Should also have special events now
    let events = subscriber.get_events().await;
    let special_count = events.iter()
        .filter(|e| e.event_type() == "test.special")
        .count();
    
    assert!(special_count > 0, "Should have special events after reconfiguration");
    
    // Disconnect the adapter
    adapter.disconnect().await?;
    
    Ok(())
}

/// Test multiple adapters publishing to the same event bus
#[tokio::test]
async fn test_multiple_adapters() -> Result<()> {
    // Create event bus and subscriber
    let event_bus = Arc::new(EventBus::new(100));
    let receiver = event_bus.subscribe();
    let subscriber = Arc::new(crate::test_harness::TestSubscriber::new("multi_test", receiver));
    subscriber.start_collecting().await;
    
    // Create three adapters with different names
    let adapter1 = {
        let base = zelan_lib::adapters::base::BaseAdapter::new("adapter1", event_bus.clone());
        Arc::new(MockAdapter::new(base))
    };
    
    let adapter2 = {
        let base = zelan_lib::adapters::base::BaseAdapter::new("adapter2", event_bus.clone());
        Arc::new(MockAdapter::new(base))
    };
    
    let adapter3 = {
        let base = zelan_lib::adapters::base::BaseAdapter::new("adapter3", event_bus.clone());
        Arc::new(MockAdapter::new(base))
    };
    
    // Connect all adapters
    adapter1.connect().await?;
    adapter2.connect().await?;
    adapter3.connect().await?;
    
    // Manually publish events from each adapter
    adapter1.publish_test_event("event1").await?;
    adapter2.publish_test_event("event2").await?;
    adapter3.publish_test_event("event3").await?;
    
    // Wait a bit for events to be processed
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Check that we received events from all adapters
    let events = subscriber.get_events().await;
    
    let sources: Vec<&str> = events.iter().map(|e| e.source()).collect();
    assert!(sources.contains(&"adapter1"), "Should have events from adapter1");
    assert!(sources.contains(&"adapter2"), "Should have events from adapter2");
    assert!(sources.contains(&"adapter3"), "Should have events from adapter3");
    
    // Disconnect all adapters
    adapter1.disconnect().await?;
    adapter2.disconnect().await?;
    adapter3.disconnect().await?;
    
    Ok(())
}

/// Simple mock adapter for testing multiple adapters
struct MockAdapter {
    base: zelan_lib::adapters::base::BaseAdapter,
}

impl MockAdapter {
    fn new(base: zelan_lib::adapters::base::BaseAdapter) -> Self {
        Self { base }
    }
    
    async fn publish_test_event(&self, event_id: &str) -> Result<()> {
        let payload = json!({
            "event_id": event_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.base.publish_event("mock.event", payload).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ServiceAdapter for MockAdapter {
    async fn connect(&self) -> Result<()> {
        self.base.set_connected(true);
        Ok(())
    }
    
    async fn disconnect(&self) -> Result<()> {
        self.base.set_connected(false);
        Ok(())
    }
    
    fn is_connected(&self) -> bool {
        self.base.is_connected()
    }
    
    fn get_name(&self) -> &str {
        self.base.name()
    }
}
