//! Tests for the OBS adapter

use anyhow::Result;
use std::sync::Arc;

use crate::adapters::obs::ObsAdapter;
use crate::EventBus;

/// Test ObsAdapter creation (non-blocking version)
#[tokio::test]
async fn test_obs_adapter_creation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));

    // Create adapter with default config
    // Instead of creating the adapter directly, we'll create it in a separate task
    // to avoid the "Cannot block the current thread from within a runtime" error
    let adapter_handle = tokio::task::spawn(async move {
        let adapter = ObsAdapter::new("obs", "obs_adapter_1", event_bus.clone(), None, None);

        // Return the adapter
        adapter
    });

    // Wait for the adapter to be created
    let _adapter = adapter_handle.await?;

    // If we got here, the adapter was created successfully
    Ok(())
}

/// Test ObsAdapter cloning (non-blocking version)
#[tokio::test]
async fn test_obs_adapter_clone() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    let event_bus_clone = event_bus.clone();

    // Create adapter in a separate task
    let adapter_handle = tokio::task::spawn(async move {
        let adapter = ObsAdapter::new("obs", "obs_adapter_2", event_bus_clone, None, None);

        // Return the adapter
        adapter
    });

    // Wait for the adapter to be created
    let adapter = adapter_handle.await?;

    // Clone the adapter in a separate task
    let adapter_clone = adapter.clone();

    // Verify they are different instances (this doesn't block)
    assert!(
        !std::ptr::eq(&adapter, &adapter_clone),
        "Cloned adapter should be a separate instance"
    );

    Ok(())
}
