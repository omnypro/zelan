//! Tests for the Twitch API client
//!
//! This module contains tests for Twitch API functionality including
//! channel and stream information fetching.

use anyhow::Result;
use std::sync::Arc;
use twitch_api::types::{UserId, UserName};
use twitch_oauth2::{AccessToken, ClientId, UserToken};

use super::test_helpers::{cleanup_twitch_env_vars, setup_twitch_env_vars};
use crate::adapters::http_client::mock::MockHttpClient;
use crate::adapters::twitch_api::TwitchApiClient;

// Set up environment variables for all tests
fn setup_env_vars() {
    setup_twitch_env_vars();
    println!("API test: TWITCH_CLIENT_ID set to: test_client_id");
}

// This function directly returns a client ID for testing without using env vars
fn get_test_client_id() -> ClientId {
    ClientId::new("test_client_id".to_string())
}

#[tokio::test]
async fn test_fetch_channel_info() -> Result<()> {
    // Set up environment variables first
    setup_env_vars();

    // Sample response data based on Twitch API documentation
    let channel_data = serde_json::json!({
        "data": [{
            "broadcaster_id": "123456",
            "broadcaster_login": "testchannel",
            "broadcaster_name": "TestChannel",
            "broadcaster_language": "en",
            "game_id": "12345",
            "game_name": "Test Game",
            "title": "Test Stream Title",
            "delay": 0,
            "tags": ["test1", "test2"],
            "content_classification_labels": ["test_label1", "test_label2"],
            "is_branded_content": false
        }]
    });

    // Create a mock HTTP client
    let mut mock_client = MockHttpClient::new();

    // Setup mock response for channel info
    mock_client.mock_success_json(
        "https://api.twitch.tv/helix/channels?broadcaster_id=123456",
        &channel_data,
    )?;

    // Create API client with our mock
    let api_client = TwitchApiClient::with_http_client(Arc::new(mock_client));

    // Create a fake client_id for testing using our helper
    let client_id = get_test_client_id();

    // Create fake credentials for testing - in real use cases these would come from the API
    let login = UserName::new("testuser".to_string());
    let user_id = UserId::new("123456".to_string());

    // For tests, we can use from_existing_unchecked
    // IMPORTANT: This is test-only code, don't use this in production!
    let token = UserToken::from_existing_unchecked(
        AccessToken::new("test_token".to_string()),
        None, // refresh_token
        client_id.clone(),
        None, // client_secret
        login,
        user_id,
        Some(Vec::new()),                           // scopes
        Some(std::time::Duration::from_secs(3600)), // expires_in
    );

    // Call the method we're testing with the direct client_id parameter
    let channel_info = api_client
        .fetch_channel_info_with_client_id(&token, "123456", client_id)
        .await?;

    // Verify we got the expected result
    assert!(channel_info.is_some());
    let info = channel_info.unwrap();

    // Convert to strings for comparison since the twitch_api types don't implement PartialEq with &str
    assert_eq!(info.broadcaster_id.as_str(), "123456");
    assert_eq!(info.broadcaster_name.as_str(), "TestChannel");
    assert_eq!(info.title, "Test Stream Title");

    // Clean up environment
    cleanup_twitch_env_vars();
    Ok(())
}

#[tokio::test]
async fn test_fetch_stream_info() -> Result<()> {
    // Set up environment variables first
    setup_env_vars();

    // Sample response data based on Twitch API documentation
    let stream_data = serde_json::json!({
        "data": [{
            "id": "123456789",
            "user_id": "123456",
            "user_login": "testchannel",
            "user_name": "TestChannel",
            "game_id": "12345",
            "game_name": "Test Game",
            "type": "live",
            "title": "Test Stream Title",
            "viewer_count": 123,
            "started_at": "2023-01-01T12:00:00Z",
            "language": "en",
            "thumbnail_url": "https://static-cdn.jtvnw.net/previews-ttv/live_user_testchannel-{width}x{height}.jpg",
            "tags": ["test1", "test2"], // Add missing tags field
            "tag_ids": ["123", "456"],
            "is_mature": false
        }]
    });

    // Create a mock HTTP client
    let mut mock_client = MockHttpClient::new();

    // Setup mock response for stream info
    mock_client.mock_success_json(
        "https://api.twitch.tv/helix/streams?user_id=123456",
        &stream_data,
    )?;

    // Create API client with our mock
    let api_client = TwitchApiClient::with_http_client(Arc::new(mock_client));

    // Create a fake client_id for testing using our helper
    let client_id = get_test_client_id();

    // Create fake credentials for testing - in real use cases these would come from the API
    let login = UserName::new("testuser".to_string());
    let user_id = UserId::new("123456".to_string());

    // For tests, we can use from_existing_unchecked
    // IMPORTANT: This is test-only code, don't use this in production!
    let token = UserToken::from_existing_unchecked(
        AccessToken::new("test_token".to_string()),
        None, // refresh_token
        client_id.clone(),
        None, // client_secret
        login,
        user_id,
        Some(Vec::new()),                           // scopes
        Some(std::time::Duration::from_secs(3600)), // expires_in
    );

    // Call the method we're testing with the direct client_id parameter
    let stream_info = api_client
        .fetch_stream_info_with_client_id(&token, Some("123456"), None, client_id)
        .await?;

    // Verify we got the expected result
    assert!(stream_info.is_some());
    let info = stream_info.unwrap();

    // Convert to strings for comparison since the twitch_api types don't implement PartialEq with &str
    assert_eq!(info.user_id.as_str(), "123456");
    assert_eq!(info.user_name.as_str(), "TestChannel");
    assert_eq!(info.title, "Test Stream Title");
    assert_eq!(info.viewer_count, 123);

    // Clean up environment
    cleanup_twitch_env_vars();
    Ok(())
}
