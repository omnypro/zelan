//! Shared test utilities for adapter tests
//!
//! This module provides common utilities used across adapter tests.

use std::collections::HashMap;

/// Constants for environment variables used by Twitch-related tests
pub const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";
pub const TWITCH_CLIENT_SECRET_ENV: &str = "TWITCH_CLIENT_SECRET";
pub const TWITCH_REDIRECT_URI_ENV: &str = "TWITCH_REDIRECT_URI";

/// Set up environment variables used by multiple tests
pub fn setup_twitch_env_vars() {
    std::env::set_var(TWITCH_CLIENT_ID_ENV, "test_client_id");
    std::env::set_var(TWITCH_CLIENT_SECRET_ENV, "test_client_secret");
    std::env::set_var(TWITCH_REDIRECT_URI_ENV, "http://localhost:8080/callback");
}

/// Clean up test environment variables
pub fn cleanup_twitch_env_vars() {
    std::env::remove_var(TWITCH_CLIENT_ID_ENV);
    std::env::remove_var(TWITCH_CLIENT_SECRET_ENV);
    std::env::remove_var(TWITCH_REDIRECT_URI_ENV);
}

/// Test wrapper to setup and cleanup environment variables
pub fn with_twitch_env_vars<F, R>(test_fn: F) -> R
where
    F: FnOnce() -> R,
{
    setup_twitch_env_vars();
    let result = test_fn();
    cleanup_twitch_env_vars();
    result
}

/// Create test configuration for Twitch adapters
pub fn create_test_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    config.insert("client_id".to_string(), "test_client_id".to_string());
    config.insert(
        "client_secret".to_string(),
        "test_client_secret".to_string(),
    );
    config.insert(
        "redirect_uri".to_string(),
        "http://localhost:8080/callback".to_string(),
    );
    config
}
