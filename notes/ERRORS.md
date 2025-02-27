# Zelan Error Reference

This document catalogs common errors, troubleshooting steps, and debugging strategies for the Zelan application.

## Authentication Errors

### Twitch Device Code Authentication

The device code authentication flow is particularly sensitive to timing and state management. Here are common issues and solutions:

#### Device Code Flow Issues

1. **Unrecognized Completion**
   - **Symptom**: User completes authentication on Twitch, but app doesn't recognize it
   - **Cause**: Race condition between polling and state updates
   - **Solution**: Ensure polling interval is consistent (5 seconds recommended) and all polling operations complete before checking state
   - **Test Status**: ✅ Basic state transition tests added

2. **Token Refresh Failures**
   - **Symptom**: Authentication works initially but fails after token expiration
   - **Cause**: Refresh token handling issues or improper error handling
   - **Solution**: Verify refresh token is properly stored and used in `refresh_token_if_needed`
   - **Test Status**: ❌ Tests needed for refresh flow

3. **Inconsistent Authentication State**
   - **Symptom**: App shows conflicting authentication status
   - **Cause**: State variable updates out of sync with actual authentication state
   - **Solution**: Use proper synchronization with RwLock and ensure all state updates happen atomically
   - **Test Status**: ✅ Basic state transition tests added

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

2. Check for these specific errors:
   - "expired_token" - The device code expired before authentication was completed
   - "authorization_pending" - Normal during polling, indicates user hasn't completed auth yet
   - "slow_down" - Polling too frequently, increase interval
   - "invalid_grant" - Problem with the refresh token

#### Authentication Testing Status

1. **Implemented Tests**:
   - ✅ Device code auth state transitions
   - ✅ Token restoration error handling
   - ✅ HTTP client abstraction for testability

2. **Tests Needed**:
   - Complete device code flow with mock HTTP responses (next step)
   - Token refresh with mocked responses 
   - Error handling for various auth errors (expired token, network failures, etc.)
   - Race condition detection
   - Timing sensitivity tests with simulated delays

3. **Testing Infrastructure**:
   - ✅ `HttpClient` trait for abstracting HTTP requests
   - ✅ `ReqwestHttpClient` implementation for real requests
   - ✅ `MockHttpClient` for testing with predetermined responses
   - ✅ Request history tracking for verification in tests
   - ✅ JSON response mocking capabilities

## API Connection Errors

### Rate Limiting

1. **Twitch API Rate Limits**
   - **Symptom**: HTTP 429 responses
   - **Solution**: Implement proper backoff with the circuit breaker pattern

### Network Failures

1. **Transient Connection Issues**
   - **Symptom**: Random connection timeouts or resets
   - **Solution**: Use the retry mechanism with exponential backoff

## Application Errors

### Adapter Initialization

1. **Adapter Not Found**
   - **Symptom**: "Adapter 'x' not found" errors
   - **Cause**: Typo in adapter name or adapter not registered
   - **Solution**: Verify adapter names match registration exactly

2. **Adapter Connection Failures**
   - **Symptom**: "Failed to connect adapter" errors
   - **Cause**: External API unavailable or authentication issue
   - **Solution**: Check external service status and credentials

## Debugging Strategies

### Enabling Debug Logging

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

### Troubleshooting Steps

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