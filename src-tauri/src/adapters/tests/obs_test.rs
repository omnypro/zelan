//! Tests for the OBS adapter

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

use crate::adapters::base::AdapterConfig;
use crate::adapters::obs::{ObsAdapter, ObsConfig};
use crate::adapters::obs_callback::ObsEvent;
use crate::EventBus;
use crate::ServiceAdapter;

/// Test ObsAdapter configuration
#[tokio::test]
async fn test_obs_adapter_config() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));

    // Create adapter with default config
    let adapter = ObsAdapter::new(event_bus.clone());

    // Get the default config
    let default_config = adapter.get_config().await;

    // Verify default values
    assert_eq!(default_config.host, "localhost");
    assert_eq!(default_config.port, 4455);
    assert_eq!(default_config.password, "");
    assert!(default_config.auto_connect);
    assert!(default_config.include_scene_details);

    // Create a custom config
    let custom_config = ObsConfig {
        host: "127.0.0.1".to_string(),
        port: 5000,
        password: "secret".to_string(),
        auto_connect: false,
        include_scene_details: false,
    };

    // Create adapter with custom config
    let custom_adapter = ObsAdapter::with_config(event_bus.clone(), custom_config.clone());

    // Get the config and verify it matches what we set
    let config = custom_adapter.get_config().await;
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 5000);
    assert_eq!(config.password, "secret");
    assert_eq!(config.auto_connect, false);
    assert_eq!(config.include_scene_details, false);

    // Test config serialization/deserialization
    let json_config = json!({
        "host": "example.com",
        "port": 4444,
        "password": "password123",
        "auto_connect": true,
        "include_scene_details": false
    });

    // Deserialize
    let deserialized = ObsConfig::from_json(&json_config)?;
    assert_eq!(deserialized.host, "example.com");
    assert_eq!(deserialized.port, 4444);
    assert_eq!(deserialized.password, "password123");
    assert!(deserialized.auto_connect);
    assert!(!deserialized.include_scene_details);

    // Test partial config update
    let partial_json = json!({
        "host": "partial-update.com",
        "port": 8080
    });

    // Deserialize partial config (should keep defaults for unspecified fields)
    let partial = ObsConfig::from_json(&partial_json)?;
    assert_eq!(partial.host, "partial-update.com");
    assert_eq!(partial.port, 8080);
    assert_eq!(partial.password, ""); // Default
    assert!(partial.auto_connect); // Default
    assert!(partial.include_scene_details); // Default

    Ok(())
}

/// Test ObsAdapter event triggering
#[tokio::test]
async fn test_obs_adapter_events() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));

    // Create adapter
    let adapter = ObsAdapter::new(event_bus.clone());

    // Set up a test OBS event
    let scene_name = "Test Scene";
    let scene_data = json!({
        "scene_name": scene_name,
        "timestamp": "2023-09-01T12:34:56Z"
    });

    let test_event = ObsEvent::SceneChanged {
        scene_name: scene_name.to_string(),
        data: scene_data.clone(),
    };

    // Set up a callback count tracking variable
    let callback_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback_count_clone = callback_count.clone();

    // Register a callback
    adapter
        .register_obs_callback(move |event| {
            // Increment the counter
            callback_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // Verify we got the right event
            match &event {
                ObsEvent::SceneChanged { scene_name, data } => {
                    assert_eq!(scene_name, "Test Scene");
                    assert_eq!(data, &scene_data);
                }
                _ => panic!("Wrong event type received"),
            }

            Ok(())
        })
        .await?;

    // Trigger the event
    adapter.trigger_event(test_event).await?;

    // Verify the callback was called
    assert_eq!(callback_count.load(std::sync::atomic::Ordering::SeqCst), 1);

    Ok(())
}

/// Test that ObsAdapter::clone() properly preserves shared state
#[tokio::test]
async fn test_obs_adapter_clone() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));

    // Create adapter with custom config
    let custom_config = ObsConfig {
        host: "192.168.1.1".to_string(),
        port: 9000,
        password: "password".to_string(),
        auto_connect: false,
        include_scene_details: false,
    };

    let adapter = ObsAdapter::with_config(event_bus.clone(), custom_config);

    // Clone the adapter
    let cloned_adapter = adapter.clone();

    // Verify they are different instances but with shared state
    assert!(
        !std::ptr::eq(&adapter, &cloned_adapter),
        "Cloned adapter should be a separate instance"
    );

    // Get and verify config from clone
    let clone_config = cloned_adapter.get_config().await;
    assert_eq!(clone_config.host, "192.168.1.1");
    assert_eq!(clone_config.port, 9000);

    // Modify config through original
    adapter
        .configure(json!({
            "host": "updated-host.com",
            "port": 1234
        }))
        .await?;

    // Verify changes are reflected in clone
    let updated_clone_config = cloned_adapter.get_config().await;
    assert_eq!(updated_clone_config.host, "updated-host.com");
    assert_eq!(updated_clone_config.port, 1234);

    // Set up a callback counter
    let callback_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback_count_clone = callback_count.clone();

    // Register callback with original adapter
    adapter
        .register_obs_callback(move |_| {
            callback_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await?;

    // Create test event
    let test_event = ObsEvent::ConnectionChanged {
        connected: true,
        data: json!({"test": true}),
    };

    // Trigger event through cloned adapter
    cloned_adapter.trigger_event(test_event).await?;

    // Verify callback was triggered
    assert_eq!(
        callback_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "Callback registered with original adapter should be triggered through clone"
    );

    Ok(())
}
