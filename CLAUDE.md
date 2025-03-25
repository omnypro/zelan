# Zelan Project Guide

## Project Overview
Zelan is a locally-hosted data aggregation service for streaming platforms that unifies data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API. It's built with Rust for the backend (using Tauri) and TypeScript/React for the frontend.

## Recent Changes
We implemented an HTTP client abstraction to improve testability:
- Created `HttpClient` trait with `SimpleHttpResponse` for standardized responses
- Built `ReqwestHttpClient` for real HTTP requests and `MockHttpClient` for testing
- Refactored `TwitchApiClient` to use dependency injection with the HTTP client
- Added tests for TwitchApiClient using mock responses
- Fixed the recovery system tests by adjusting expected counter values

We also completed the frontend refactoring:
- Split monolithic App.tsx into modular components
- Implemented useReducer pattern with typed actions
- Created custom hooks for data fetching and adapter control
- Added proper TypeScript interfaces for all data structures
- Designed desktop-style UI components (status indicators, notifications)

We fixed critical issues with callback integrity:
- Fixed EventSub activation not happening after initial authentication
- Enhanced Clone implementations to maintain state and callback integrity
- Added proper Arc wrapping for shared state across async boundaries
- Documented patterns for maintaining reactive integrity

## Build Commands
- `bun dev` - Start Vite development server
- `bun tauri dev` - Start Tauri app in development mode
- `bun build` - Build the frontend (TypeScript + Vite)
- `bun tauri build` - Build the full Tauri application
- `cargo test` - Run Rust tests in the src-tauri directory
- `cargo test [test_name]` - Run a specific Rust test
- `cargo clippy` - Run Rust linter
- `cargo fmt` - Format Rust code

## Environment Setup
This project requires environment variables for certain features to work properly.

### Twitch Integration
- Requires the `TWITCH_CLIENT_ID` environment variable to be set in a `.env` file
- Uses device code flow authentication (suitable for desktop applications)
- Only requests minimum necessary scopes for read operations
- See `.env.example` for reference and instructions to obtain a client ID

## Architecture

### Adapter System
- **BaseAdapter**: Foundation for all service adapters with common functionality
- **ServiceAdapter**: Core interface all adapters must implement
- Each adapter handles a specific platform:
  - **TwitchAdapter**: Connects to Twitch API and monitors channel/stream events
  - **ObsAdapter**: Connects to OBS via WebSockets for scene information
  - **TestAdapter**: Generates test events for debugging

### WebSocket Server
- Provides real-time event streaming to external clients
- Configurable with customizable settings:
  - Port configuration (default: 8080)
  - Connection limits (default: 100 simultaneous connections)
  - Timeout settings (default: 5 minutes of inactivity)
  - Ping intervals (default: 60 seconds)
- Client subscription system for filtering events:
  - Filter by event source (e.g., only Twitch events)
  - Filter by event type (e.g., only stream.online events)
- Standard event format with versioning for backward compatibility
- Efficient event serialization with caching for performance

### Event-Driven Design
- Adapters publish events to a central EventBus
- Events are propagated to subscribers (UI components, other services)
- Async/await patterns used for non-blocking operations
- WebSocket server provides real-time event delivery to external clients
- Enhanced event filtering system with subscription capabilities
- Support for 13 different Twitch EventSub event types

### Reactive Architecture
- Events trigger callbacks through well-defined channels
- Authentication events trigger feature activation (e.g., EventSub)
- Callbacks preserved across Clone boundaries through proper Arc sharing
- Token refreshing automatically triggers appropriate state updates
- Centralized EventBus for all internal communication

### Frontend Architecture
- **Component-Based Structure**:
  - Modular components (Dashboard, Settings, ErrorNotification, etc.)
  - Desktop-style UI elements for native look and feel
  - Status indicators with visual feedback
- **State Management**:
  - useReducer pattern with typed actions
  - Centralized AppState with well-defined interfaces
  - Clear separation of UI state and application data
- **Custom Hooks**:
  - useTauriCommand: Safe invocation of backend commands
  - useAdapterControl: Adapter management operations
  - useDataFetching: Data retrieval from backend
  - useAppState: Application state management

## Code Style Guidelines

### TypeScript/React
- Use React functional components with hooks
- Prefer explicit type annotations with TypeScript
- Use async/await for asynchronous operations
- Group related imports together (React, internal, external)
- Follow 2-space indentation
- Split large components into smaller, focused ones
- Use custom hooks to encapsulate specific functionality
- Prefer useReducer for complex state management
- Separate UI components from data fetching/business logic
- Design with desktop-friendly UI/UX patterns in mind

### Rust
- Follow standard Rust naming conventions (snake_case for functions/variables)
- Use the `anyhow` crate for error handling
- Implement proper error propagation with `?` operator
- Use async/await for asynchronous code with Tokio runtime
- Favor Arc<RwLock<T>> for shared mutable state
- Implement Clone trait for components requiring shared ownership
- Follow 4-space indentation
- Use structs and traits for adapter interfaces

### Callback and Event Integrity
- **CRITICAL**: When implementing Clone for adapters with callbacks, ensure the same callback instances are shared by using Arc::clone()
- Never create new instances of objects that hold callbacks in Clone implementations
- Wrap all shared state (RwLock, Mutex) in Arc to maintain shared access across clones
- Always share the same instances of managers that hold callbacks (auth_manager, event_manager, etc.)
- Use the pattern `x: Arc::clone(&self.x)` rather than creating new locks
- Ensure reactive patterns preserve callback integrity across async tasks and clones
- Be careful with event propagation across clone boundaries - events should reach all callback handlers
- When debugging reactive system failures, check if events are correctly propagating to registered callbacks

#### Proper Struct Design for Cloneable Types with Shared State
- Always wrap RwLock and Mutex fields in Arc if the containing type might be cloned
- Follow this pattern for field declarations:
  ```rust
  struct MyType {
      // Good - wrapped in Arc for sharing across clones
      field1: Arc<RwLock<SomeType>>, 
      field2: Arc<Mutex<OtherType>>,
      
      // Bad - will create separate locks for each clone
      bad_field: RwLock<SomeType>, 
  }
  ```
- Implement Clone for types that might be shared across async tasks
- For Clone implementations with callbacks, always use Arc::clone() to share the same callback instances:
  ```rust
  impl Clone for MyType {
      fn clone(&self) -> Self {
          Self {
              field1: Arc::clone(&self.field1),
              field2: Arc::clone(&self.field2),
              // ... other fields
          }
      }
  }
  ```
- Document the Clone behavior in comments to prevent future regressions

### Event System Best Practices
- Prefer reactive event-driven patterns over direct method calls for better decoupling
- Use the EventBus for all inter-component communication
- Create strongly-typed event definitions with clear documentation
- Register callbacks at component initialization time
- Use explicit event types rather than generic strings when possible
- Implement diagnostic logging for event propagation in debug/development mode
- Add event tracing for easier debugging of reactive chains
- Ensure all callbacks are properly maintained across clone boundaries
- Avoid duplicating event handlers - register them once and ensure proper sharing
- When a reactive path isn't working, trace the entire event chain to find the break

### Twitch Integration
- NEVER use `UserToken::from_existing_unchecked()`. Always use `UserToken::from_existing()` instead which performs proper validation.
- Always ensure token expiration times are properly tracked and stored in TokenManager.
- When working with tokens, make sure they're fully validated before use.
- Prefer direct token validation through the Twitch API over constructing tokens manually.
- Always check the expiration of refresh tokens (30-day limit for device code flow tokens).
- When refreshing tokens, ensure the new expiration time is properly stored.

### Authentication
- Prefer OAuth device code flow for desktop applications
- Request minimal scopes necessary for functionality
- Handle token refreshing and restoration automatically
- Store tokens securely using Tauri's secure storage when possible

### General
- Document public APIs and complex functions
- Handle errors explicitly rather than panicking
- Prefer the simplest solution that uses what the libraries provide
- Do not recreate functionality unless absolutely necessary
- Write tests for critical components
- Use cargo fmt and appropriate linters before committing

### Documentation
- Documentation for all Rust crates can be found in the /targets/doc directory