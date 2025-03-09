# Zelan Project Roadmap

This roadmap outlines the planned development priorities for the Zelan data aggregation service.

## Current Status

The project currently has a functional foundation with:

- Reactive event system using RxJS (EventBus/EventStream)
- TestAdapter implementation and adapter framework
- Configuration/persistence layer for settings
- Structured logging system with Winston
- Type-safe IPC using tRPC
- Demo interfaces for testing core functionality
- Tailwind and shadcn/ui integration

## Development Priorities

### 1. Core Systems Completion

#### WebSocket Server Implementation

- [ ] Create external API for clients with WebSockets
- [ ] Implement real-time event streaming
- [ ] Add authentication and connection management
- [ ] Define and document the API protocol

#### Authentication System

- [ ] Migrate Twitch authentication from src-old
- [ ] Implement TokenManager with secure storage
- [ ] Create auth UI components and state transitions
- [ ] Add token refresh and expiration handling
- [ ] Build authentication provider interface for different services

### 2. Service Adapters

#### Twitch Adapter

- [ ] Implement authentication flow using @twurple libraries
- [ ] Add channel/stream events monitoring
- [ ] Create configuration UI

#### OBS Adapter

- [ ] Implement WebSocket connection to OBS using obs-websocket-js
- [ ] Add scene switching and status monitoring
- [ ] Create configuration UI

#### Test Adapter Enhancements

- [ ] Add more event types and customization options
- [ ] Create stress testing capabilities
- [ ] Implement advanced simulation patterns

### 3. Technical Improvements

#### RxJS Data Transformation

- [ ] Create custom operators for event processing
- [ ] Build event pipelines for aggregating data across sources
- [ ] Implement caching and filtering mechanisms
- [ ] Add data transformation utilities for common patterns

#### Error Handling & Resilience

- [ ] Add automatic reconnection for subscriptions
- [ ] Implement error boundaries in React components
- [ ] Create centralized error reporting
- [x] Add structured logging system for diagnostics
- [ ] Implement log rotation and management
- [ ] Create log viewer UI for diagnostics
- [ ] Implement health check mechanisms

#### Type Safety Enhancements

- [ ] Replace all remaining `any` types with proper interfaces
- [ ] Add Zod validation for cross-process data
- [ ] Create runtime type checking for external inputs

#### Subscription Management

- [ ] Build a centralized subscription registry
- [ ] Add lifecycle management for subscriptions
- [ ] Implement memory leak prevention

### 4. Architecture Refinements

#### State Management

- [ ] Implement a formalized Observable Store pattern
- [ ] Add selective updates to minimize re-renders
- [ ] Create command/query separation

#### IPC Optimization

- [ ] Optimize serialization for large data payloads
- [ ] Add compression for high-volume events
- [ ] Implement batching for frequent updates

#### Event Cache & Transformation

- [ ] Create a sophisticated event caching system
- [ ] Implement event transformation pipelines
- [ ] Add filtering capabilities for clients

### 5. User Experience

#### UI Improvements

- [ ] Replace placeholder UI with shadcn/ui components
- [ ] Create proper dashboard layout with responsive design
- [ ] Implement adapter configuration screens
- [ ] Add event visualization components
- [ ] Create service status indicators
- [ ] Implement dark/light theme support

#### Desktop Notifications

- [ ] Implement system notifications for important events
- [ ] Add customizable notification rules

#### Event Analytics

- [ ] Add event statistics and metrics collection
- [ ] Implement historical data storage
- [ ] Create data visualization components
- [ ] Add filtering and search capabilities for events
- [ ] Implement event aggregation and summary features

### 6. Developer Experience

#### Testing Framework

- [ ] Build unit tests for tRPC-RxJS integration
- [ ] Create subscription testing utilities
- [ ] Implement end-to-end testing
- [ ] Add test coverage reporting
- [ ] Set up CI/CD pipeline for automated testing

#### Logging & Diagnostics

- [x] Implement structured logging with Winston
- [x] Create component-specific loggers
- [ ] Add log filtering by component and level
- [ ] Implement log export functionality
- [ ] Create a log viewer UI component
- [ ] Add log search and analysis features

#### Documentation

- [ ] Document the WebSocket API for external clients
- [x] Document logging best practices
- [x] Document module import guidelines
- [ ] Create comprehensive internal API documentation
- [ ] Add architecture diagrams
- [ ] Create example client implementations in different languages

#### Development Tools

- [ ] Implement dev tools for event monitoring
- [ ] Create a configuration editor
- [ ] Add performance profiling

### 7. Deployment & Distribution

#### Installer & Auto-updates

- [ ] Create proper installer packages
- [ ] Implement auto-update mechanism
- [ ] Add version management

#### Configuration Migration

- [ ] Build tools for migrating configurations between versions
- [ ] Add backward compatibility layer

## Future Considerations

- Custom plugin system for third-party extensions
- Scheduled events and triggers
- Local recording/archiving of event data
- Data export functionality
- Multiple workspace support
- Mobile companion app

## Implementation Strategy

This roadmap represents a comprehensive view of the project's direction. Implementation will be prioritized based on:

1. Core functionality requirements
2. Technical stability considerations
3. User experience priorities

The suggested initial focus would be:

1. WebSocket Server Implementation
2. Authentication System
3. Service-Specific Adapters
4. Technical Improvements

Technical improvements will be integrated alongside feature development to maintain code quality and prevent accumulation of technical debt.
