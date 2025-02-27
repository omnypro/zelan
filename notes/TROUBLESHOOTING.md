# Zelan Troubleshooting Guide

This document provides guidance for troubleshooting common issues, error patterns, and debugging strategies for the Zelan application.

## Authentication Issues

### Twitch Device Code Authentication

The device code authentication flow is particularly sensitive to timing and state management:

#### Common Issues

1. **Unrecognized Authentication Completion**
   - **Symptom**: User completes authentication on Twitch, but app doesn't recognize it
   - **Cause**: Race condition between polling and state updates
   - **Solution**: Ensure polling interval is consistent (5 seconds recommended) and all polling operations complete before checking state

2. **Token Refresh Failures**
   - **Symptom**: Authentication works initially but fails after token expiration
   - **Cause**: Refresh token handling issues or improper error handling
   - **Solution**: Verify refresh token is properly stored and used in `refresh_token_if_needed`

3. **Inconsistent Authentication State**
   - **Symptom**: App shows conflicting authentication status
   - **Cause**: State variable updates out of sync with actual authentication state
   - **Solution**: Use proper synchronization with RwLock and ensure all state updates happen atomically

#### Debugging Authentication

For effective authentication debugging:

1. Add detailed logging at key points:
   ```rust
   // Before initiating device code flow
   tracing::debug!("Starting device code flow with scopes: {:?}", scopes);
   
   // After receiving device code
   tracing::debug!("Received device code: verification_uri={}, user_code={}, expires_in={}s",
       device_code.verification_uri, device_code.user_code, device_code.expires_in);
   
   // During polling
   tracing::debug!("Polling for auth completion: poll_count={}", poll_count);
   
   // On successful authentication
   tracing::info!("Successfully authenticated, token expires in {}s", token.expires_in);
   
   // On authentication errors
   tracing::warn!("Authentication error: {:?}", err);
   ```

2. Check for these specific error codes:
   - `expired_token`: The device code expired before authentication was completed
   - `authorization_pending`: Normal during polling, indicates user hasn't completed auth yet
   - `slow_down`: Polling too frequently, increase interval
   - `invalid_grant`: Problem with the refresh token

### Token Refresh Issues

#### Device Code Flow Limitations

1. **One-Time Use Refresh Tokens**: Refresh tokens obtained via Device Code Flow are one-time use only. When used to refresh an access token, they become invalid and a new refresh token is provided.

2. **30-Day Expiry**: There is a 30-day inactive expiry on refresh tokens. If a refresh token is not used for 30 days, it expires and the user must authenticate again.

3. **Access Token Expiry**: All access tokens have a 4-hour expiry time.

#### Automatic Background Refresh Implementation

We've implemented these enhancements to handle token limitations transparently:

1. **Token Tracking**: When refreshing tokens, we check if a new refresh token is received as expected.

2. **30-Day Expiry Prevention**: We track when refresh tokens were created and perform automatic background refreshes before they reach the 30-day expiry limit.

3. **Proactive Refresh**: We refresh tokens before they expire to reset the 30-day inactivity window.

4. **Token Synchronization**: New tokens from refreshes are automatically persisted to secure storage.

#### Key Error Patterns

- **invalid_grant**: Often indicates the refresh token has been used before or has expired.
- **authorization_pending**: Normal status during device code flow authorization process.
- **expired_token**: The device code or refresh token has expired.

## API Connection Issues

### Rate Limiting

1. **Twitch API Rate Limits**
   - **Symptom**: HTTP 429 responses
   - **Solution**: Implement proper backoff with the circuit breaker pattern

### Network Failures

1. **Transient Connection Issues**
   - **Symptom**: Random connection timeouts or resets
   - **Solution**: Use the retry mechanism with exponential backoff

## Adapter Issues

### Adapter Initialization

1. **Adapter Not Found**
   - **Symptom**: "Adapter 'x' not found" errors
   - **Cause**: Typo in adapter name or adapter not registered
   - **Solution**: Verify adapter names match registration exactly

2. **Adapter Connection Failures**
   - **Symptom**: "Failed to connect adapter" errors
   - **Cause**: External API unavailable or authentication issue
   - **Solution**: Check external service status and credentials

## Enabling Debug Logging

To enable debug logging:

```rust
// In main.rs
tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("zelan=debug".parse().unwrap()))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

Or set environment variable:
```
RUST_LOG=zelan=debug
```

## General Troubleshooting Steps

1. **Authentication Issues**:
   - Check if tokens are being stored correctly
   - Verify refresh tokens are working properly
   - Confirm scopes are correct for the operations being performed

2. **API Connectivity**:
   - Test direct API access with curl or Postman
   - Verify network connectivity
   - Check for API status issues on Twitch status page

3. **Event Flow Issues**:
   - Ensure event bus is properly initialized
   - Check for event subscribers
   - Verify event types match expectations