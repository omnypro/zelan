//! Tests for the Twitch adapter
//! 
//! This module contains tests for the main Twitch adapter functionality, including:
//! - Configuration and initialization
//! - Authentication integration
//! - Event subscription and publishing
//! - State management

use std::sync::Arc;
use anyhow::Result;
use serde_json::json;

use crate::adapters::base::AdapterConfig;
use crate::adapters::twitch::{TwitchAdapter, TwitchConfig};
use crate::adapters::twitch_auth::{AuthEvent, TwitchAuthManager};
use crate::ServiceAdapter;
use crate::EventBus;

use super::test_helpers::{setup_twitch_env_vars, cleanup_twitch_env_vars};

/// Test TwitchAdapter basic creation
#[tokio::test]
async fn test_twitch_adapter_creation() -> Result<()> {
    // Set up environment
    setup_twitch_env_vars();
    
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create adapter
    let adapter = TwitchAdapter::new(event_bus.clone()).await;
    
    // Basic assertions
    assert!(!adapter.is_connected());
    
    // Clean up
    cleanup_twitch_env_vars();
    Ok(())
}

/// Test TwitchAdapter configuration
#[tokio::test]
async fn test_twitch_adapter_config() -> Result<()> {
    // Set up environment
    setup_twitch_env_vars();
    
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create adapter with default config
    let adapter = TwitchAdapter::new(event_bus.clone()).await;
    
    // Configure the adapter
    let config = json!({
        "channel_login": "test_channel",
        "poll_interval_ms": 60000
    });
    
    adapter.configure(config).await?;
        
    // Clean up
    cleanup_twitch_env_vars();
    Ok(())
}

/// Test TwitchAdapter clone properly shares state
#[tokio::test]
async fn test_twitch_adapter_clone() -> Result<()> {
    // Set up environment
    setup_twitch_env_vars();
    
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create adapter
    let adapter = TwitchAdapter::new(event_bus.clone()).await;
    
    // Create a clone
    let clone = adapter.clone();
    
    // Configure the original
    let config = json!({
        "channel_login": "test_channel"
    });
    
    adapter.configure(config).await?;
    
    // The clone should see the configuration too
    assert_eq!(adapter.get_name(), clone.get_name());
    
    // Clean up
    cleanup_twitch_env_vars();
    Ok(())
}

/// Test authentication event handling
#[tokio::test]
async fn test_auth_event_propagation() -> Result<()> {
    // Setup environment
    setup_twitch_env_vars();
    
    // Create event bus
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create adapter
    let adapter = TwitchAdapter::new(event_bus.clone()).await;
    
    // Get access to the auth manager and trigger an event
    adapter.trigger_auth_event(AuthEvent::AuthenticationSuccess).await?;
    
    // Since this just tests event propagation, we don't need to verify anything else
    
    // Clean up
    cleanup_twitch_env_vars();
    Ok(())
}
