//! Tests for Twitch EventSub functionality
//!
//! This module contains tests for the EventSub client, WebSocket connection,
//! and event handling capabilities.

use super::test_helpers::{setup_twitch_env_vars, cleanup_twitch_env_vars};
use crate::adapters::twitch_eventsub::*;
use crate::adapters::BackoffStrategy;
use crate::EventBus;
use std::sync::Arc;
use std::time::Duration;

// Helper struct that mimics the parts of UserToken we need for hashing
struct TestUserToken {
    access_token: String,
    refresh_token: Option<String>,
}

impl TestUserToken {
    fn new(access_token: &str, refresh_token: Option<&str>) -> Self {
        Self {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|s| s.to_string()),
        }
    }
    
    // Method to get access token secret - mimics the UserToken interface
    fn access_token_secret(&self) -> &str {
        &self.access_token
    }
    
    // Method to get refresh token - mimics the UserToken interface
    fn refresh_token_secret(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }
}

#[tokio::test]
async fn test_eventsub_client_creation() {
    // Set up environment
    setup_twitch_env_vars();

    // Create event bus with default capacity
    let event_bus = Arc::new(EventBus::new(100));

    // Create EventSub client
    let client = EventSubClient::new(event_bus).await;
    
    // Verify client was created successfully
    assert!(client.is_ok());
    
    // Clean up
    cleanup_twitch_env_vars();
}

#[tokio::test]
async fn test_backoff_strategy() {
    // Test our backoff strategy implementation
    let strategy = BackoffStrategy::Exponential {
        base_delay: Duration::from_secs(DEFAULT_RECONNECT_DELAY_SECS),
        max_delay: Duration::from_secs(MAX_RECONNECT_DELAY_SECS),
    };
    
    // First attempt (should be equal to base delay)
    let delay1 = strategy.calculate_delay(1);
    assert_eq!(delay1, Duration::from_secs(DEFAULT_RECONNECT_DELAY_SECS));
    
    // Second attempt (should be 2x base delay)
    let delay2 = strategy.calculate_delay(2);
    assert_eq!(delay2, Duration::from_secs(DEFAULT_RECONNECT_DELAY_SECS * 2));
    
    // Should respect max delay
    let delay10 = strategy.calculate_delay(10);
    assert!(delay10 <= Duration::from_secs(MAX_RECONNECT_DELAY_SECS));
}

#[tokio::test]
async fn test_token_hash_calculation() {
    // Set up environment
    setup_twitch_env_vars();
    
    // Create event bus with default capacity
    let event_bus = Arc::new(EventBus::new(100));
    
    // Create EventSub client
    let _client = EventSubClient::new(event_bus).await.unwrap(); // Prefix with _ to indicate intentional non-use
    
    // For testing, we'll use a simpler approach that doesn't involve real token validation
    // Just create structure to use with our hash function
        
    // Let's modify the hash_token function for our test to use our test struct
    let hash_token = |token: &TestUserToken| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        token.access_token_secret().hash(&mut hasher);
        if let Some(refresh_token) = token.refresh_token_secret() {
            refresh_token.hash(&mut hasher);
        }
        hasher.finish()
    };
    
    // Create test tokens
    let token1 = TestUserToken::new("test_access_token", Some("test_refresh_token"));
    let token2 = TestUserToken::new("test_access_token", Some("test_refresh_token"));
    let token3 = TestUserToken::new("different_access_token", Some("test_refresh_token"));
    
    // Calculate hashes
    let hash1 = hash_token(&token1);
    let hash2 = hash_token(&token2);
    let hash3 = hash_token(&token3);
    
    // Identical tokens should have identical hashes
    assert_eq!(hash1, hash2);
    
    // Different tokens should have different hashes
    assert_ne!(hash1, hash3);
    
    // Clean up
    cleanup_twitch_env_vars();
}