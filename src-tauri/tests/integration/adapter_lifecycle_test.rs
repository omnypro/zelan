//! Integration tests for adapter lifecycle
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde_json::json;

use zelan_lib::adapters::test::{TestAdapter, TestConfig};
use zelan_lib::EventBus;

/// Simplified test that demonstrates adapter creation
#[tokio::test]
async fn test_adapter_creation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(zelan_lib::EventBus::new(100));

    // Create adapters
    let _test_adapter = zelan_lib::adapters::test::TestAdapter::new(
        "test",
        "test_adapter_1",
        event_bus.clone(),
        None,
    );

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}
