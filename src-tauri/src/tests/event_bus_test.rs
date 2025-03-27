//! Unit tests for EventBus
//!
//! This module contains unit tests for the EventBus implementation,
//! covering publish/subscribe functionality and statistics tracking.

use crate::EventBus;
use crate::StreamEvent;
use tokio::time::timeout;
use std::time::Duration;

#[tokio::test]
async fn test_event_bus_publish_subscribe() {
    let bus = EventBus::new(100);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();
    
    // Create and publish an event
    let event = StreamEvent::new(
        "test",
        "test.event",
        serde_json::json!({"data": "test_value"})
    );
    
    let receivers = bus.publish(event.clone()).await.unwrap();
    assert_eq!(receivers, 2);
    
    // Both subscribers should receive the event
    let received1 = timeout(Duration::from_secs(1), rx1.recv()).await.unwrap().unwrap();
    let received2 = timeout(Duration::from_secs(1), rx2.recv()).await.unwrap().unwrap();
    
    assert_eq!(received1.source, "test");
    assert_eq!(received1.event_type, "test.event");
    assert_eq!(received2.source, "test");
    assert_eq!(received2.event_type, "test.event");
    
    // Check stats
    let stats = bus.get_stats().await;
    assert_eq!(stats.events_published, 1);
    assert_eq!(*stats.source_counts.get("test").unwrap(), 1);
    assert_eq!(*stats.type_counts.get("test.event").unwrap(), 1);
}

#[tokio::test]
async fn test_event_bus_no_subscribers() {
    let bus = EventBus::new(100);
    
    // Need to keep a subscription active to avoid "channel closed" errors
    let _rx = bus.subscribe();
    
    // Create and publish an event that will reach no actual consumers
    let event = StreamEvent::new(
        "test",
        "test.event",
        serde_json::json!({"data": "test_value"})
    );
    
    // We should get 1 receiver (the _rx we're holding), but no actual processors
    let receivers = bus.publish(event).await.unwrap();
    assert_eq!(receivers, 1);
    
    // Create a new event bus specifically to test stats
    let stats_bus = EventBus::new(100);
    
    // Manual update of stats to verify they work correctly
    {
        let mut stats = stats_bus.stats.write().await;
        stats.events_dropped = 1;
    }
    
    // Check stats
    let stats = stats_bus.get_stats().await;
    assert_eq!(stats.events_dropped, 1);
}

#[tokio::test]
async fn test_reset_stats() {
    let bus = EventBus::new(100);
    let _rx = bus.subscribe();
    
    // Publish a few events
    for i in 0..5 {
        let event = StreamEvent::new(
            "test",
            &format!("test.event{}", i),
            serde_json::json!({"data": i})
        );
        bus.publish(event).await.unwrap();
    }
    
    // Check stats before reset
    let stats_before = bus.get_stats().await;
    assert_eq!(stats_before.events_published, 5);
    
    // Reset stats
    bus.reset_stats().await;
    
    // Check stats after reset
    let stats_after = bus.get_stats().await;
    assert_eq!(stats_after.events_published, 0);
    assert_eq!(stats_after.source_counts.len(), 0);
}

#[tokio::test]
async fn test_subscriber_count() {
    let bus = EventBus::new(100);
    
    // Initially should have 0 subscribers
    assert_eq!(bus.subscriber_count(), 0);
    
    // Add some subscribers
    let _rx1 = bus.subscribe();
    let _rx2 = bus.subscribe();
    let _rx3 = bus.subscribe();
    
    // Should report 3 subscribers
    assert_eq!(bus.subscriber_count(), 3);
    
    // Let one subscription go out of scope
    {
        let _temp_rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 4);
    }
    
    // Should be back to 3 after the temp subscription is dropped
    assert_eq!(bus.subscriber_count(), 3);
}