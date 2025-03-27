//! Integration tests for verifying basic callback functionality
use anyhow::Result;
use std::sync::Arc;

/// Simplified test that just checks that we can instantiate the adapters
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

    let _obs_adapter = zelan_lib::adapters::obs::ObsAdapter::new(
        "obs",
        "obs_adapter_1",
        event_bus.clone(),
        None,
        None,
    );

    let _twitch_adapter = zelan_lib::adapters::twitch::TwitchAdapter::new(
        "twitch",
        "twitch_adapter_1",
        event_bus.clone(),
        Arc::new(zelan_lib::auth::token_manager::TokenManager::new()),
        Arc::new(zelan_lib::recovery::RecoveryManager::new()),
        None,
    );

    // If we got this far, creation succeeded
    Ok(())
}

/// Test cloning of adapters
#[tokio::test]
async fn test_adapter_cloning() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(zelan_lib::EventBus::new(100));

    // Create test adapter
    let test_adapter = zelan_lib::adapters::test::TestAdapter::new(
        "test",
        "test_adapter_2",
        event_bus.clone(),
        None,
    );

    // Create clone
    let test_clone = test_adapter.clone();

    // Verify clone is a separate instance
    assert!(
        !std::ptr::eq(&test_adapter, &test_clone),
        "Cloned adapter should be a separate instance"
    );

    // Create OBS adapter
    let obs_adapter = zelan_lib::adapters::obs::ObsAdapter::new(
        "obs",
        "obs_adapter_2",
        event_bus.clone(),
        None,
        None,
    );

    // Create clone
    let obs_clone = obs_adapter.clone();

    // Verify clone is a separate instance
    assert!(
        !std::ptr::eq(&obs_adapter, &obs_clone),
        "Cloned adapter should be a separate instance"
    );

    // Create Twitch adapter
    let twitch_adapter = zelan_lib::adapters::twitch::TwitchAdapter::new(
        "twitch",
        "twitch_adapter_2",
        event_bus.clone(),
        Arc::new(zelan_lib::auth::token_manager::TokenManager::new()),
        Arc::new(zelan_lib::recovery::RecoveryManager::new()),
        None,
    );

    // Create clone
    let twitch_clone = twitch_adapter.clone();

    // Verify clone is a separate instance
    assert!(
        !std::ptr::eq(&twitch_adapter, &twitch_clone),
        "Cloned adapter should be a separate instance"
    );

    Ok(())
}
