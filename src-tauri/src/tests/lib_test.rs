//! Tests for the library's core functionality
//!
//! This module contains simple tests for core library components
//! that don't require extensive test setup.

use crate::EventBus;

/// Simple test for the EventBus creation and subscription
#[tokio::test]
async fn test_event_bus_simple() {
    let bus = EventBus::new(100);
    assert_eq!(bus.subscriber_count(), 0);

    let _rx = bus.subscribe();
    assert_eq!(bus.subscriber_count(), 1);
}
