# Zelan Development Notes

This document contains technical notes, decisions, and implementation details about the Zelan project.

## Authentication Refactoring

### Authentication System Design Decisions

The Twitch authentication flow uses the device code flow, which is ideal for desktop applications:

- **Device Code Flow**: Doesn't require a web server callback, suitable for desktop apps
- **Minimal Scopes**: Only requests the minimum necessary permissions
- **One-Time Use Refresh Tokens**: Refresh tokens are used once and replaced
- **30-Day Expiry**: Refresh tokens expire after 30 days of inactivity

### Authentication Implementation Guidelines

When working with the authentication system, remember:

- **State Coherence**: Device codes and auth states must be properly synchronized
- **Testing First**: Always write tests before changing authentication flow
- **Recovery Mechanisms**: Include code to recover from partial auth flows
- **Clean Documentation**: Document auth sequences clearly
- **Incremental Changes**: Make small, well-tested modifications rather than rewrites

### Twitch API Client Testing

We've implemented a robust testing approach for Twitch API Client:

- **HttpClient Abstraction**: Created a trait for HTTP operations
- **Mock HTTP Client**: Implemented for testing with predetermined responses
- **Request History**: Tracks requests for verification in tests
- **Environment Independence**: Tests don't rely on environment variables
- **Test-Specific Methods**: Created overloads that accept direct parameters

## Frontend Architecture Decisions

### Component Structure Rationale

We opted for a component-based architecture to improve maintainability:

- **Small, Focused Components**: Each component has a single responsibility
- **Separation of Concerns**: UI components separate from business logic
- **Desktop-Style UI**: Components designed to feel like a native desktop app
- **Consistent Patterns**: Similar problems solved in similar ways

### State Management Approach

We chose useReducer over Redux for state management:

- **Simpler Implementation**: No need for external libraries
- **TypeScript Integration**: Better type safety with TypeScript
- **Predictable Updates**: Actions provide clear intent
- **Centralized Logic**: State changes are handled in one place

### Custom Hooks Strategy

We created several custom hooks to encapsulate functionality:

- **useTauriCommand**: Abstracts backend communication
- **useAdapterControl**: Handles adapter operations
- **useDataFetching**: Manages data retrieval and caching
- **useAppState**: Provides access to application state

## Error Handling Approach

### Error Classification System

We implemented a comprehensive error classification system:

- **Error Categories**: Network, Authentication, RateLimit, etc.
- **Error Severity**: Info, Warning, Error, Critical
- **Error Context**: Additional information for debugging
- **Error Registry**: Centralized tracking of errors

### Circuit Breaker Implementation

The circuit breaker pattern prevents cascading failures:

- **Failure Threshold**: Number of failures before opening circuit
- **Reset Timeout**: Time to wait before trying again
- **Half-Open State**: Testing if the service is healthy again
- **Atomic Operations**: Thread-safe state management

## Testing Strategy

### Test Coverage Goals

We aim for comprehensive test coverage:

- **Unit Tests**: For individual components and functions
- **Integration Tests**: For component interactions
- **Mock-Based Tests**: For external dependencies
- **Property-Based Tests**: For complex logic
- **UI Tests**: For frontend components

### Test Implementation Approach

We've standardized our testing approach:

- **Test First**: Write tests before implementing features
- **Mock External Services**: Use mock implementations for testing
- **Environment Independence**: Tests don't rely on environment variables
- **Comprehensive Coverage**: Test both success and failure paths
- **Clear Assertions**: Tests clearly state what they're verifying