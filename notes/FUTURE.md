# Zelan Future Development Roadmap

## Completed Items

### ✅ 1. Replace Polling with Twitch EventSub

The Twitch adapter now uses EventSub instead of polling, which:
- Provides real-time event notifications
- Reduces API usage and latency
- Is more efficient and reliable than polling
- Delivers events immediately when they occur

### ✅ 2. Consolidate Token Management

Token management has been centralized through a dedicated TokenManager that:
- Provides a single source of truth for token state
- Handles secure token storage and retrieval
- Manages token refreshing
- Offers a unified interface for all adapters

### ✅ 3. Implement a Robust Error Recovery System

A comprehensive error recovery system has been implemented that:
- Provides retry policies with exponential backoff for different error types
- Includes a circuit breaker pattern for persistent failures
- Classifies errors by type (network, auth, rate limit) with appropriate handling
- Enhances logging with detailed error contexts and categories
- Creates a centralized error registry for debugging and analysis
- Offers adapter-specific recovery strategies through the AdapterRecovery trait

## Current Priorities

### 1. Codebase Simplification and Refactoring

The codebase has grown complex and needs simplification to improve maintainability:

#### 1.1 Completed Simplifications
- ✅ Implemented error builder pattern for cleaner error creation
- ✅ Used `VecDeque` for proper FIFO error history
- ✅ Integrated `thiserror` for better typed errors
- ✅ Improved circuit breaker with atomic types and cleaner API
- ✅ Created simplified recovery system with better abstractions

#### 1.2 Remaining Backend Improvements
- Extract Twitch authentication into a separate reusable component (✅ partially implemented, but requires careful testing)
- ⚠️ **Authentication Warning**: The Twitch device code flow is sensitive to implementation details. If refactoring, make minimal changes and test extensively.
- Simplify token recovery logic into a single method
- Consider using `tracing-error` for better error context
- Consolidate retry logic into a unified approach
- Consider `tokio-retry` for retry implementation

#### 1.3 TypeScript/Frontend Improvements
- Split App.tsx into smaller components (Dashboard, Settings, etc.)
- Extract reusable UI components and patterns
- Replace multiple useState calls with useReducer for related state
- Create custom hooks for adapter control and Tauri commands
- Implement proper TypeScript interfaces instead of `any` types
- Standardize component patterns for consistency
- Use a tab-based router pattern for main UI sections

### 2. Add Comprehensive Testing Suite

The codebase needs better test coverage for reliability and maintainability:

#### 2.1 Authentication Testing (Highest Priority)

Due to recent authentication regressions, this is now our highest testing priority:

- **Create Authentication Regression Tests** ✅
  - Basic tests implemented for auth state transitions and error handling
  - Currently tests:
    - Device code flow state transitions (NotAuthenticated → PendingDeviceAuth → NotAuthenticated)
    - Token restoration error handling
  - Added tests within the standard `#[cfg(test)]` module in the source file

- **Additional Authentication Tests Needed**
  - Test the complete flow including authenticated state
  - Cover more error cases (timeouts, expired codes, network failures)
  - Test token refresh logic
  - ⚠️ **Critical**: Complete test coverage before further auth refactoring

- **Build a Robust Mock System** ✅ In Progress
  - ✅ Developed a `HttpClient` trait for abstracting HTTP interactions
  - ✅ Implemented `ReqwestHttpClient` for real HTTP requests
  - ✅ Created `MockHttpClient` for testing with predefined responses
  - ✅ Added request history tracking for verification in tests
  - ✅ Implemented JSON response mocking capabilities
  - Implement delay simulation for timing-sensitive code
  - Build a comprehensive suite of mock Twitch API responses
  - Design the mock system to be reusable across all adapters

#### 2.2 Complete Testing System

After authentication testing is established, expand to all components:
- Create unit tests for all adapter components
- Add integration tests for end-to-end flows
- Implement mocks for other external APIs (OBS)
- Ensure tests verify both the happy path and error recovery paths
- Create a CI pipeline for continuous testing

### 3. Enhance WebSocket Client Support

Better client libraries would improve the developer experience:

Recommendation: Create client SDKs for different languages:
- Develop a JavaScript client library for web overlays
- Create a simple Python client for integration with other tools
- Add TypeScript types for all events
- Implement reconnection logic in clients
- Add examples of common integration patterns

## Future Enhancements

### 4. Improve Documentation

Better documentation would make the project more accessible:

Recommendation: Enhance documentation coverage:
- Create comprehensive API documentation
- Add more examples and quickstart guides
- Document the event schema for consumers
- Create tutorials for common integration scenarios
- Add troubleshooting guides

### 5. Implement Proper Data Persistence

The current approach to storing configuration and tokens could be improved:

Recommendation: Enhance data persistence architecture:
- Create a structured database (SQLite) for configuration
- Implement proper migrations for configuration changes
- Add versioning to stored settings
- Create a backup/restore system for user configuration
- Use a transaction-based approach for all storage operations

### 6. Add a Pluggable Extension System

A plugin system would allow for more flexibility and extensibility:

Recommendation: Create a plugin architecture for adapters:
- Implement a proper plugin system for dynamic loading
- Allow third-party adapters to be installed
- Create a standardized API for all adapters
- Add a plugin marketplace or directory
- Support hot-reloading of adapters

### 7. Implement Advanced Event Processing

The event system could be enhanced with more powerful features:

Recommendation: Enhance the event system:
- Add event filtering based on type/source
- Implement event transformations (combining, splitting)
- Create an event history with searchable logs
- Add event replay capabilities for debugging
- Support for conditional events and complex routing

### 8. Add Better Diagnostics and Monitoring

Better monitoring would improve troubleshooting and reliability:

Recommendation: Create advanced diagnostics:
- Implement a performance monitoring dashboard
- Add detailed metrics for API calls, event throughput
- Create visualization of event flows
- Add system health checks with alerts
- Implement periodic connection testing for all adapters

### 9. Improve UI/UX

Enhancing the user interface would make the application more user-friendly:

Recommendation: Enhance the user experience:
- Create a more intuitive configuration interface
- Add real-time status indicators for adapters
- Implement a dashboard with event visualization
- Add theme support (light/dark mode)
- Improve responsiveness for different screen sizes

### 10. Add Deployment and Update Mechanisms

Better deployment options would make the application easier to distribute:

Recommendation: Enhance deployment capabilities:
- Add auto-update functionality
- Create installers for multiple platforms
- Consider containerization options
- Implement a headless mode for server deployments
- Add configuration export/import

This roadmap addresses both immediate improvements and long-term enhancements, with a focus on making Zelan more robust, developer-friendly, and extensible.
