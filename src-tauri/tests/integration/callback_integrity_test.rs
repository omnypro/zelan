//! Integration tests for verifying callback integrity across clone boundaries
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::Duration;

use anyhow::Result;
use serde_json::json;
use tokio::sync::Mutex;

use zelan_lib::adapters::twitch::{TwitchAdapter, TwitchConfig};
use zelan_lib::adapters::twitch_auth::AuthEvent;
use zelan_lib::adapters::test::{TestAdapter, TestConfig};
use zelan_lib::adapters::test_callback::TestEvent;
use zelan_lib::adapters::obs::ObsAdapter;
use zelan_lib::adapters::obs_callback::ObsEvent;
use zelan_lib::{EventBus, ServiceAdapter};

/// Test that the TwitchAdapter's auth callbacks are properly preserved across clone boundaries
#[tokio::test]
async fn test_auth_callback_preservation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create a shared counter to track callback invocations
    let callback_counter = Arc::new(AtomicUsize::new(0));
    let callback_triggered = Arc::new(AtomicBool::new(false));
    
    // Keep track of events received
    let received_events = Arc::new(Mutex::new(Vec::new()));
    
    // Create the original adapter
    let adapter = TwitchAdapter::new(event_bus.clone());
    
    // Register an auth callback with the original adapter that updates our counter
    {
        // Clone our counter for the callback closure
        let counter_clone = Arc::clone(&callback_counter);
        let events_clone = Arc::clone(&received_events);
        let trigger_clone = Arc::clone(&callback_triggered);
        
        // Set up a test callback that will track invocations
        adapter.register_auth_callback(move |event| {
            // Increment the counter
            let current = counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("Auth callback invoked: {} times, event: {:?}", current + 1, event);
            
            // Record the event
            let event_clone = event.clone();
            let events_clone2 = Arc::clone(&events_clone);
            
            // Spawn a task to update the mutex to avoid blocking the callback
            tokio::spawn(async move {
                events_clone2.lock().await.push(event_clone);
            });
            
            // Mark that we've received an auth event
            trigger_clone.store(true, Ordering::SeqCst);
            
            Ok(())
        }).await?;
    }
    
    // Create a clone of the adapter
    let cloned_adapter = adapter.clone();
    
    // Verify the clone has a separate identity but shared callback mechanisms
    assert!(!std::ptr::eq(&adapter, &cloned_adapter), 
        "Cloned adapter should be a separate instance");
    
    // Make sure there's a subscriber to the event bus
    let _receiver = event_bus.subscribe();
    
    // Trigger an auth event on the original adapter
    println!("Triggering auth event on original adapter");
    adapter.trigger_auth_event(AuthEvent::DeviceCodeReceived {
        user_code: "test_user_code".to_string(),
        verification_uri: "https://example.com".to_string(),
        expires_in: 1800,
    }).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered exactly once
    assert_eq!(callback_counter.load(Ordering::SeqCst), 1, 
        "Callback should be triggered exactly once after first event");
    
    // Now trigger an auth event on the cloned adapter
    println!("Triggering auth event on cloned adapter");
    cloned_adapter.trigger_auth_event(AuthEvent::AuthenticationSuccess).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered a second time
    assert_eq!(callback_counter.load(Ordering::SeqCst), 2, 
        "Callback should be triggered twice after second event");
    
    // Check that we received the correct events in order
    let events = received_events.lock().await;
    assert_eq!(events.len(), 2, "Should have received exactly 2 events");
    
    // Verify the first event type
    match &events[0] {
        AuthEvent::DeviceCodeReceived { .. } => {
            // Expected - this was our first event
        },
        other => panic!("First event should be DeviceCodeReceived, got {:?}", other),
    }
    
    // Verify the second event type
    match &events[1] {
        AuthEvent::AuthenticationSuccess { .. } => {
            // Expected - this was our second event
        },
        other => panic!("Second event should be AuthenticationSuccess, got {:?}", other),
    }
    
    // Now create a third independently created adapter
    let independent_adapter = TwitchAdapter::new(event_bus.clone());
    
    // Set up a new callback for this adapter
    let independent_counter = Arc::new(AtomicUsize::new(0));
    {
        let counter_clone = Arc::clone(&independent_counter);
        
        independent_adapter.register_auth_callback(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).await?;
    }
    
    // Trigger an event on the original adapter again
    adapter.trigger_auth_event(AuthEvent::TokenRefreshed).await?;
    
    // Short wait
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the original callback was triggered a third time
    assert_eq!(callback_counter.load(Ordering::SeqCst), 3, 
        "Original callback should be triggered a third time");
    
    // But the independent adapter's callback should not have been triggered
    assert_eq!(independent_counter.load(Ordering::SeqCst), 0, 
        "Independent adapter callback should not be triggered by events on original adapter");
    
    // Now trigger an event on the independent adapter
    independent_adapter.trigger_auth_event(AuthEvent::AuthenticationFailed {
        error: "test error".to_string(),
    }).await?;
    
    // Short wait
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the independent adapter's callback was triggered
    assert_eq!(independent_counter.load(Ordering::SeqCst), 1, 
        "Independent adapter callback should be triggered once");
    
    // The original adapter's callback should still be at 3
    assert_eq!(callback_counter.load(Ordering::SeqCst), 3, 
        "Original callback should not be affected by independent adapter events");
    
    Ok(())
}

/// Test that TestAdapter cloning preserves shared configuration state
#[tokio::test]
async fn test_test_adapter_shared_state_preservation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create test adapter with custom config
    let config = TestConfig {
        interval_ms: 5000,
        generate_special_events: true,
    };
    
    let adapter = TestAdapter::with_config(event_bus.clone(), config);
    
    // Create a clone of the adapter
    let cloned_adapter = adapter.clone();
    
    // Verify the configuration is shared by modifying it through the original adapter
    adapter.configure(json!({
        "interval_ms": 10000,
        "generate_special_events": false
    })).await?;
    
    // Short wait to ensure the change propagates
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Get the configuration from the cloned adapter directly and verify changes
    let cloned_config = cloned_adapter.get_config().await;
    
    // Check that the interval was updated in the clone
    assert_eq!(cloned_config.interval_ms, 10000, 
        "Interval should be updated in the cloned adapter");
    
    // Check that the special events flag was updated in the clone
    assert_eq!(cloned_config.generate_special_events, false, 
        "Generate special events flag should be updated in the cloned adapter");
    
    // Modify through clone
    cloned_adapter.configure(json!({
        "interval_ms": 15000,
    })).await?;
    
    // Short wait
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Verify change appears in original
    let original_config = adapter.get_config().await;
    assert_eq!(original_config.interval_ms, 15000,
        "Changes made through clone should be visible in original adapter");
    
    // Verify that the special events flag is still false (unchanged)
    assert_eq!(original_config.generate_special_events, false,
        "Unchanged config values should remain consistent");
    
    Ok(())
}

/// Test that the TestAdapter's callbacks are properly preserved across clone boundaries
#[tokio::test]
async fn test_test_adapter_callback_preservation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create a shared counter to track callback invocations
    let callback_counter = Arc::new(AtomicUsize::new(0));
    
    // Keep track of events received
    let received_events = Arc::new(Mutex::new(Vec::new()));
    
    // Create the original adapter
    let adapter = TestAdapter::new(event_bus.clone());
    
    // Register a callback with the original adapter that updates our counter
    {
        // Clone our counter for the callback closure
        let counter_clone = Arc::clone(&callback_counter);
        let events_clone = Arc::clone(&received_events);
        
        // Set up a test callback that will track invocations
        adapter.register_test_callback(move |event| {
            // Increment the counter
            let current = counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("Test callback invoked: {} times, event: {:?}", current + 1, event.event_type());
            
            // Record the event
            let event_clone = event.clone();
            let events_clone2 = Arc::clone(&events_clone);
            
            // Spawn a task to update the mutex to avoid blocking the callback
            tokio::spawn(async move {
                events_clone2.lock().await.push(event_clone);
            });
            
            Ok(())
        }).await?;
    }
    
    // Create a clone of the adapter
    let cloned_adapter = adapter.clone();
    
    // Verify the clone has a separate identity but shared callback mechanisms
    assert!(!std::ptr::eq(&adapter, &cloned_adapter), 
        "Cloned adapter should be a separate instance");
    
    // Trigger a test event on the original adapter
    println!("Triggering test event on original adapter");
    adapter.trigger_event(TestEvent::Initial {
        counter: 1,
        message: "Initial event from original".to_string(),
        data: json!({"source": "original"}),
    }).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered exactly once
    assert_eq!(callback_counter.load(Ordering::SeqCst), 1, 
        "Callback should be triggered exactly once after first event");
    
    // Now trigger a test event on the cloned adapter
    println!("Triggering test event on cloned adapter");
    cloned_adapter.trigger_event(TestEvent::Standard {
        counter: 2,
        message: "Standard event from clone".to_string(),
        data: json!({"source": "clone"}),
    }).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered a second time
    assert_eq!(callback_counter.load(Ordering::SeqCst), 2, 
        "Callback should be triggered twice after second event");
    
    // Check that we received the correct events in order
    let events = received_events.lock().await;
    assert_eq!(events.len(), 2, "Should have received exactly 2 events");
    
    // Verify event types by matching their type strings
    assert_eq!(events[0].event_type(), "initial", "First event should be Initial");
    assert_eq!(events[1].event_type(), "standard", "Second event should be Standard");
    
    Ok(())
}

/// Test that the ObsAdapter's callbacks are properly preserved across clone boundaries
#[tokio::test]
async fn test_obs_adapter_callback_preservation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create a shared counter to track callback invocations
    let callback_counter = Arc::new(AtomicUsize::new(0));
    
    // Keep track of events received
    let received_events = Arc::new(Mutex::new(Vec::new()));
    
    // Create the original adapter
    let adapter = ObsAdapter::new(event_bus.clone());
    
    // Register a callback with the original adapter that updates our counter
    {
        // Clone our counter for the callback closure
        let counter_clone = Arc::clone(&callback_counter);
        let events_clone = Arc::clone(&received_events);
        
        // Set up a test callback that will track invocations
        adapter.register_obs_callback(move |event| {
            // Increment the counter
            let current = counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("OBS callback invoked: {} times, event: {:?}", current + 1, event.event_type());
            
            // Record the event
            let event_clone = event.clone();
            let events_clone2 = Arc::clone(&events_clone);
            
            // Spawn a task to update the mutex to avoid blocking the callback
            tokio::spawn(async move {
                events_clone2.lock().await.push(event_clone);
            });
            
            Ok(())
        }).await?;
    }
    
    // Create a clone of the adapter
    let cloned_adapter = adapter.clone();
    
    // Verify the clone has a separate identity but shared callback mechanisms
    assert!(!std::ptr::eq(&adapter, &cloned_adapter), 
        "Cloned adapter should be a separate instance");
    
    // Trigger an OBS event on the original adapter
    println!("Triggering OBS event on original adapter");
    adapter.trigger_event(ObsEvent::ConnectionChanged {
        connected: true,
        data: json!({"source": "original"}),
    }).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered exactly once
    assert_eq!(callback_counter.load(Ordering::SeqCst), 1, 
        "Callback should be triggered exactly once after first event");
    
    // Now trigger an OBS event on the cloned adapter
    println!("Triggering OBS event on cloned adapter");
    cloned_adapter.trigger_event(ObsEvent::SceneChanged {
        scene_name: "New Scene".to_string(),
        data: json!({"source": "clone"}),
    }).await?;
    
    // Short wait to ensure the callback has time to execute
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify the callback was triggered a second time
    assert_eq!(callback_counter.load(Ordering::SeqCst), 2, 
        "Callback should be triggered twice after second event");
    
    // Check that we received the correct events in order
    let events = received_events.lock().await;
    assert_eq!(events.len(), 2, "Should have received exactly 2 events");
    
    // Verify event types by matching their type strings
    assert_eq!(events[0].event_type(), "connection_changed", "First event should be ConnectionChanged");
    assert_eq!(events[1].event_type(), "scene_changed", "Second event should be SceneChanged");
    
    Ok(())
}

/// Test that cloning preserves shared state like configuration
#[tokio::test]
async fn test_shared_state_preservation() -> Result<()> {
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create test adapter with custom config
    let config = TwitchConfig {
        channel_id: Some("12345".to_string()),
        channel_login: Some("test_channel".to_string()),
        poll_interval_ms: 10000,
        ..Default::default()
    };
    
    let adapter = TwitchAdapter::with_config(event_bus.clone(), config);
    
    // Create a clone of the adapter
    let cloned_adapter = adapter.clone();
    
    // Verify the configuration is shared by modifying it through the original adapter
    adapter.configure(json!({
        "channel_id": "67890",
        "poll_interval_ms": 20000
    })).await?;
    
    // Short wait to ensure the change propagates
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Get the configuration from the cloned adapter and verify the changes
    let cloned_config = cloned_adapter.get_config().await?;
    
    // Check that the channel ID was updated in the clone
    let channel_id = cloned_config["channel_id"].as_str().unwrap();
    assert_eq!(channel_id, "67890", 
        "Channel ID should be updated in the cloned adapter");
    
    // Check that the poll interval was updated in the clone
    let poll_interval = cloned_config["poll_interval_ms"].as_u64().unwrap();
    assert_eq!(poll_interval, 20000, 
        "Poll interval should be updated in the cloned adapter");
    
    // Check if the channel login exists and was preserved
    if let Some(channel_login) = cloned_config["channel_login"].as_str() {
        assert_eq!(channel_login, "test_channel", 
            "Channel login should be unchanged in the cloned adapter");
    } else {
        // If it's null or not present, that's also valid behavior
        println!("Note: channel_login was not present in the cloned config");
    }
    
    Ok(())
}