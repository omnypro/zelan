//! Integration tests for the EventBus system
use std::time::Duration;
use anyhow::Result;
use serde_json::json;

use zelan_lib::StreamEvent;
use crate::test_harness::TestEnvironment;

/// Test basic event publishing and subscribing
#[tokio::test]
async fn test_basic_event_flow() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add two subscribers
    let _sub1 = env.add_subscriber("subscriber1").await;
    let _sub2 = env.add_subscriber("subscriber2").await;
    
    // Reset stats before starting
    env.reset_stats().await;
    
    // Start the test adapter
    env.start_adapter().await?;
    
    // Wait for some events to be generated
    let events_received = env.wait_for_events("subscriber1", 5, 1000).await?;
    assert!(events_received, "Should have received at least 5 events");
    
    // Check that subscriber2 also received events
    let events_received = env.wait_for_events("subscriber2", 5, 100).await?;
    assert!(events_received, "Second subscriber should have received events");
    
    // Stop the test adapter
    env.stop_adapter().await?;
    
    // Get stats and verify
    let published = env.get_published_count().await;
    let dropped = env.get_dropped_count().await;
    assert!(published > 0, "Should have published events");
    assert_eq!(dropped, 0, "Should not have dropped events");
    
    // Check the actual events received
    let events = env.get_subscriber_events("subscriber1").await?;
    
    // Verify event properties
    for event in events {
        assert_eq!(event.source(), "test", "Event source should be 'test'");
        assert!(
            event.event_type() == "test.event" || event.event_type() == "test.special",
            "Event type should be 'test.event' or 'test.special'"
        );
        
        // Check payload
        let payload = event.payload();
        assert!(payload.get("counter").is_some(), "Payload should contain counter");
        assert!(payload.get("message").is_some(), "Payload should contain message");
        assert!(payload.get("timestamp").is_some(), "Payload should contain timestamp");
    }
    
    Ok(())
}

/// Test multiple subscribers with different event loads
#[tokio::test]
async fn test_multiple_subscribers() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add multiple subscribers
    for i in 1..=5 {
        env.add_subscriber(&format!("subscriber{}", i)).await;
    }
    
    // Reset stats before starting
    env.reset_stats().await;
    
    // Start the test adapter
    env.start_adapter().await?;
    
    // Wait for events to be generated
    let events_received = env.wait_for_events("subscriber1", 10, 2000).await?;
    assert!(events_received, "Should have received at least 10 events");
    
    // Stop the test adapter
    env.stop_adapter().await?;
    
    // Get stats and verify all subscribers got the same events
    let events1 = env.get_subscriber_events("subscriber1").await?;
    let count1 = events1.len();
    
    for i in 2..=5 {
        let events = env.get_subscriber_events(&format!("subscriber{}", i)).await?;
        assert_eq!(
            events.len(), 
            count1, 
            "All subscribers should receive the same number of events"
        );
    }
    
    // Check stats
    let published = env.get_published_count().await;
    assert_eq!(
        published, 
        count1 as u64, 
        "Stats should match the number of events"
    );
    
    Ok(())
}

/// Test publishing custom events directly
#[tokio::test]
async fn test_custom_events() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add subscriber
    let _sub = env.add_subscriber("test_sub").await;
    
    // Reset stats
    env.reset_stats().await;
    
    // Publish custom events directly to the event bus
    let custom_event = StreamEvent::new(
        "custom", 
        "test.custom", 
        json!({
            "custom_id": 1,
            "message": "Custom test event",
            "data": {
                "foo": "bar",
                "baz": 42
            }
        })
    );
    
    let receivers = env.event_bus.publish(custom_event).await?;
    assert_eq!(receivers, 1, "Should have 1 receiver for the custom event");
    
    // Wait a bit for the event to be processed
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Check that the custom event was received
    let events = env.get_subscriber_events("test_sub").await?;
    assert_eq!(events.len(), 1, "Should have received exactly one event");
    
    let event = &events[0];
    assert_eq!(event.source(), "custom", "Event source should be 'custom'");
    assert_eq!(event.event_type(), "test.custom", "Event type should match");
    
    let payload = event.payload();
    assert_eq!(
        payload.get("custom_id").and_then(|v| v.as_u64()), 
        Some(1), 
        "Payload should contain custom_id=1"
    );
    
    // Check stats
    let published = env.get_published_count().await;
    assert_eq!(published, 1, "Should have published 1 event");
    
    Ok(())
}

/// Test handling of event overflows (when buffer is full)
#[tokio::test]
async fn test_event_overflow() -> Result<()> {
    // For this test we need a very small capacity event bus
    let event_bus = std::sync::Arc::new(zelan_lib::EventBus::new(5));
    
    // Create a subscriber but don't consume events yet
    let receiver = event_bus.subscribe();
    let subscriber = std::sync::Arc::new(crate::test_harness::TestSubscriber::new(
        "overflow_test", 
        receiver
    ));
    
    // Don't start collecting yet!
    
    // Create a bunch of events (more than capacity)
    for i in 0..20 {
        let event = StreamEvent::new(
            "test", 
            "overflow.event", 
            json!({ "index": i })
        );
        
        // It's ok if some of these fail - that's what we're testing
        let _ = event_bus.publish(event).await;
    }
    
    // Now start collecting events
    subscriber.start_collecting().await;
    
    // Publish one more event that should definitely be received
    let final_event = StreamEvent::new(
        "test", 
        "overflow.final", 
        json!({ "final": true })
    );
    
    let receivers = event_bus.publish(final_event).await?;
    assert_eq!(receivers, 1, "Should have 1 receiver for the final event");
    
    // Wait a bit for the event to be processed
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Check stats - we should have some events
    let stats_json = serde_json::to_value(&event_bus.get_stats().await).unwrap();
    let published = stats_json["events_published"].as_u64().unwrap_or(0);
    assert!(published > 0, "Should have published events");
    
    // The final event should be in the events we received
    let events = subscriber.get_events().await;
    let has_final = events.iter().any(|e| 
        e.event_type() == "overflow.final" && 
        e.payload().get("final").and_then(|v| v.as_bool()) == Some(true)
    );
    
    assert!(has_final, "Should have received the final event");
    
    Ok(())
}