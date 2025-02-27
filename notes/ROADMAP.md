# Zelan Development Roadmap

This document outlines the development roadmap for the Zelan project in priority order.

## Immediate Priorities

### 1. Complete Authentication Testing

We've made significant progress on token management improvements:

- ✅ **TokenManager Improvements**
  - Added tests for token expiration validation
  - Implemented tests for 30-day refresh token expiry logic
  - Added better error handling for token metadata parsing
  - Improved token restoration validation

- ✅ **Token Recovery Logic**
  - Created a unified `recover_tokens_with_validation` method
  - Implemented retry mechanisms with exponential backoff
  - Added proper error handling and validation
  - Improved error reporting during recovery

Work still needed:

- **Token Refresh Tests**
  - Create tests for the token refresh flow
  - Test handling of expired tokens
  - Mock HTTP responses for various authentication scenarios

- **Error Recovery Tests**
  - Test authentication error recovery paths
  - Simulate network failures during authentication
  - Test race conditions in the auth state machine

- **Edge Case Tests**
  - Test timing-sensitive code with simulated delays
  - Test the device code flow with various response types
  - Test token restoration from secure storage

### 2. Backend Simplifications

Several backend improvements are still needed:

- **Consolidate Retry Logic**
  - Evaluate using tokio-retry for retry implementation
  - Create a unified approach to retries
  - Improve circuit breaker implementation

- **Error Context Improvements**
  - Consider using tracing-error for better error context
  - Enhance error reporting for authentication issues
  - Implement better error classification

### 3. Integration Testing

We've started implementing an integration testing framework focused on the EventBus and adapter lifecycle. The current implementation includes:

- ✅ **Integration Test Framework**
  - Created a test harness with TestEnvironment and TestSubscriber
  - Implemented utilities for testing event propagation
  - Added test helpers for common operations

- ✅ **EventBus Integration Tests**
  - Implemented tests for basic event flow
  - Added tests for multiple subscribers
  - Created tests for custom events and overflow conditions
  
- ✅ **Adapter Lifecycle Tests**
  - Tested adapter connect/disconnect lifecycle
  - Implemented tests for dynamic reconfiguration
  - Added tests for multiple adapter coordination

Work still needed:

- **Authentication Integration Tests**
  - Create tests for complete authentication workflows
  - Test token refresh and recovery in integration scenarios
  - Test error recovery across adapter boundaries

- ✅ **WebSocket Server Tests**
  - Implemented WebSocket client and server test harness
  - Added tests for connection and event forwarding
  - Created tests for client reconnection scenarios
  - Implemented high-throughput tests for stress conditions
  - Added tests for server shutdown with connected clients
  - Improved test reliability with timeouts and error handling
  - Resolved state synchronization issues between tests
  - Added ping/pong protocol verification
  - Implemented concurrent connection testing

- **Add Unit Tests for Error Handling**
  - Test error classification system
  - Test retry policies with different error types
  - Test circuit breaker pattern implementation

- **Documentation and Examples**
  - Update code comments and documentation
  - Create examples for using the simplified APIs
  - Clean up unused code and imports

## Future Enhancements

### 4. Enhanced WebSocket Client Support

Better client libraries would improve the developer experience:

- Develop JavaScript client library for web overlays
- Create simple Python client for integration with other tools
- Add TypeScript types for all events
- Implement reconnection logic in clients
- Add examples of common integration patterns

### 5. Improved Documentation

Comprehensive documentation would make the project more accessible:

- Create comprehensive API documentation
- Add more examples and quickstart guides
- Document the event schema for consumers
- Create tutorials for common integration scenarios
- Add troubleshooting guides

### 6. Proper Data Persistence

The current approach to storing configuration and tokens could be improved:

- Create a structured database (SQLite) for configuration
- Implement proper migrations for configuration changes
- Add versioning to stored settings
- Create a backup/restore system
- Use a transaction-based approach for storage operations

### 7. Pluggable Extension System

A plugin system would allow for more flexibility:

- Implement a proper plugin system for dynamic loading
- Allow third-party adapters to be installed
- Create a standardized API for all adapters
- Add a plugin marketplace or directory
- Support hot-reloading of adapters

### 8. Advanced Event Processing

The event system could be enhanced with more powerful features:

- Add event filtering based on type/source
- Implement event transformations
- Create an event history with searchable logs
- Add event replay capabilities for debugging
- Support for conditional events and complex routing

### 9. Better Diagnostics and Monitoring

Better monitoring would improve troubleshooting and reliability:

- Implement a performance monitoring dashboard
- Add detailed metrics for API calls, event throughput
- Create visualization of event flows
- Add system health checks with alerts
- Implement periodic connection testing

### 10. Improved UI/UX

Enhancing the user interface would make the application more user-friendly:

- Create a more intuitive configuration interface
- Improve real-time status indicators
- Implement a dashboard with event visualization
- Add theme support (light/dark mode)
- Improve responsiveness for different screen sizes

### 11. Deployment and Updates

Better deployment options would make the application easier to distribute:

- Add auto-update functionality
- Create installers for multiple platforms
- Consider containerization options
- Implement a headless mode for server deployments
- Add configuration export/import