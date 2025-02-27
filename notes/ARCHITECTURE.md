# Zelan Architecture

This document describes the architecture of the Zelan application, including its components, interactions, and design patterns.

## System Overview

Zelan is a locally-hosted data aggregation service for streaming platforms that unifies data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API. It's built with:

- **Backend**: Rust with Tauri for native capabilities
- **Frontend**: TypeScript/React for the user interface
- **Communications**: WebSockets for real-time event streaming

## Core Components

### Adapter System

The adapter system is the foundation of Zelan's extensibility:

- **BaseAdapter**: Foundation for all service adapters with common functionality
- **ServiceAdapter**: Core interface all adapters must implement
- Each adapter handles a specific platform:
  - **TwitchAdapter**: Connects to Twitch API and monitors channel/stream events
  - **ObsAdapter**: Connects to OBS via WebSockets for scene information
  - **TestAdapter**: Generates test events for debugging

### Event Bus

The event bus is the central communication system:

- Adapters publish events to a central EventBus
- Events are propagated to subscribers (UI components, other services)
- Async/await patterns used for non-blocking operations
- Events are typed with source and category information

### Error Recovery System

The error recovery system ensures robustness:

- **ErrorRegistry**: Centralized tracking of errors
- **RetryPolicy**: Configurable retry strategies with exponential backoff
- **CircuitBreaker**: Prevents cascading failures when services are unhealthy
- **AdapterRecovery**: Adapter-specific recovery strategies

### Authentication System

The authentication system handles platform credentials:

- **TokenManager**: Centralized token storage and retrieval
- **TwitchAuthManager**: Handles Twitch OAuth device code flow
- **TwitchApiClient**: Communicates with the Twitch API using tokens
- Automatic token refresh and restoration

### HTTP Client System

The HTTP client system provides abstraction for testing:

- **HttpClient**: Trait defining standard HTTP operations
- **ReqwestHttpClient**: Real HTTP implementation using reqwest
- **MockHttpClient**: Mock implementation for testing
- Request history tracking for test verification

### WebSocket Server

The WebSocket server provides data to external consumers:

- Real-time event streaming to clients
- JSON-formatted events with standard structure
- Configurable port and security options
- HTTP API endpoint for legacy clients

## Frontend Architecture

The frontend is structured for maintainability and a desktop-like experience:

### Component Structure

- **App**: Main application container
- **Dashboard**: Monitoring interface for events and adapter status
- **Settings**: Configuration interface for adapters
- **ErrorNotification**: Desktop-style error and info notifications
- **StatusIndicator**: Visual indicator for connection status
- **WebSocketInfo**: Connection information display
- **AdapterCard**: Configuration card for each adapter

### State Management

- **useReducer**: Centralized state management with typed actions
- **AppState**: Main application state with strong typing
- **Custom Hooks**: Encapsulated functionality for specific tasks:
  - **useTauriCommand**: Safe interaction with backend
  - **useAdapterControl**: Adapter management operations
  - **useDataFetching**: Data retrieval from backend
  - **useAppState**: Application state handling

## Data Flow

1. **Adapters** connect to external services (Twitch, OBS, etc.)
2. **Events** are collected and normalized by adapters
3. **Event Bus** distributes events to internal subscribers
4. **WebSocket Server** streams events to external clients
5. **UI** displays current status and allows configuration

## Authentication Flow

1. **Device Code Flow** initiates authentication with Twitch
2. User completes authentication on Twitch website
3. **TwitchAuthManager** polls for completion
4. **TokenManager** stores obtained tokens securely
5. **TwitchAdapter** uses tokens for API access
6. Tokens are automatically refreshed when needed

## Error Handling

1. **Error Classification** categorizes errors by type
2. **RetryPolicy** determines appropriate retry strategy
3. **CircuitBreaker** prevents repeated failures
4. **UI Notifications** inform user of important errors
5. **Logging** captures detailed information for debugging