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

- ✅ **Token Refresh Tests**
  - Created tests for the token refresh flow
  - Tested handling of expired tokens
  - Mocked HTTP responses for various authentication scenarios

Work still needed:
  
- **EventSub Activation**
  - TODO: Once we authenticate, we should immediately activate EventSub
  - Implement proper WebSocket transport creation
  - Fix subscription creation to use the correct API methods
  - Ensure subscriptions are created within the 10-second window after connection

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

### 4. Expanded Adapter System

We need to expand the adapter system to support more services:

1. **Twitch Chat Adapter (Implemented)**
   - ✅ Uses EventSub WebSocket for chat messages (modern approach)
   - ✅ Event types: chat.message, chat.command, chat.bits, chat.subscription
   - ✅ Leverages existing TwitchAuthManager
   - ✅ Part of the main TwitchAdapter using EventSub

2. **Apple Music Adapter**
   - Integration with Apple Music API
   - Event types: music.playing, music.changed, music.stopped
   - Support for now playing information with artwork

3. **Rainwave.cc Adapter**
   - Integration with Rainwave radio API
   - Event types: music.playing, music.changed
   - Support for station selection and voting

4. **Potential Future Adapters**
   - Spotify Adapter for music integration
   - StreamElements/Streamlabs for donation events
   - Discord for chat bridge and voice status

### 5. Enhanced WebSocket Client Support

Better client libraries would improve the developer experience:

- Develop JavaScript client library for web overlays
- Create simple Python client for integration with other tools
- Add TypeScript types for all events
- Implement reconnection logic in clients
- Add examples of common integration patterns

### 6. Core Architecture Enhancements

Several architecture improvements would make the system more flexible:

1. **Enhanced Event Structure**
   - Add correlation IDs for related events
   - Include metadata for filtering and categorization
   - Standardize event type naming across adapters

2. **Pattern-Based Subscriptions**
   - Allow subscribing to events using patterns like "chat.*" or "*.changed"
   - Support for complex filtering rules

3. **Event History**
   - Maintain a configurable event history buffer
   - Allow querying recent events by type or pattern

4. **API Unification**
   - Standardized adapter interface with capabilities discovery
   - RESTful endpoints for all adapter operations
   - Query language for retrieving specific data

5. **WebSocket Server Improvements**
   - Authentication system for connections
   - Bidirectional communication for commands
   - Protocol improvements with compression options

### 7. Improved Documentation

Comprehensive documentation would make the project more accessible:

- Create comprehensive API documentation
- Add more examples and quickstart guides
- Document the event schema for consumers
- Create tutorials for common integration scenarios
- Add troubleshooting guides

### 8. Proper Data Persistence

The current approach to storing configuration and tokens could be improved:

- Create a structured database (SQLite) for configuration
- Implement proper migrations for configuration changes
- Add versioning to stored settings
- Create a backup/restore system
- Use a transaction-based approach for storage operations

### 9. Pluggable Extension System

A plugin system would allow for more flexibility:

- Implement a proper plugin system for dynamic loading
- Allow third-party adapters to be installed
- Create a standardized API for all adapters
- Add a plugin marketplace or directory
- Support hot-reloading of adapters

### 10. Advanced Event Processing

The event system could be enhanced with more powerful features:

- Add event filtering based on type/source
- Implement event transformations
- Create an event history with searchable logs
- Add event replay capabilities for debugging
- Support for conditional events and complex routing

### 11. Better Diagnostics and Monitoring

Better monitoring would improve troubleshooting and reliability:

- Implement a performance monitoring dashboard
- Add detailed metrics for API calls, event throughput
- Create visualization of event flows
- Add system health checks with alerts
- Implement periodic connection testing

### 12. Improved UI/UX

Enhancing the user interface would make the application more user-friendly:

- Create a more intuitive configuration interface
- Improve real-time status indicators
- Implement a dashboard with event visualization
- Add theme support (light/dark mode)
- Improve responsiveness for different screen sizes
- TODO: Add a button to deauthenticate/logout from connected services
  - Need to implement proper token revocation with services
  - Clear local token storage securely
  - Update UI state to reflect disconnected status

### 13. Deployment and Updates

Better deployment options would make the application easier to distribute:

- Add auto-update functionality
- Create installers for multiple platforms
- Consider containerization options
- Implement a headless mode for server deployments
- Add configuration export/import

### 14. Example Applications & Client Libraries

Creating example applications will showcase the platform's capabilities:

- Chat overlay with custom theming
- Music "Now Playing" display for streams
- Stream status dashboard
- JavaScript client library for web overlays
- Python client library for tools and bots
- C# client library for Unity integrations
