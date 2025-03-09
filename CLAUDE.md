# Zelan Project Guide

## Project Overview

Zelan is a lightweight, locally-hosted data aggregation service that ingests data from streaming platforms (Twitch, OBS, etc.) and exposes it through a standardized API. It enables stream overlays and third-party applications to access unified data without direct coupling to source services. We're transitioning from Rust/Tauri to an Electron/TypeScript approach to improve development velocity and simplify complex features like authentication.

## Current State of the Codebase

The project has moved beyond the initial setup phase and has implemented several core systems:

- **Core Event System**: A reactive event system using RxJS with EventBus and EventStream components
- **WebSocket Server**: Operational server for external clients to connect and receive events
- **Adapter System**: Framework with TestAdapter implementation that generates sample events
- **Configuration/Persistence Layer**: Manages settings and user preferences with RxDB (reactive database)
- **IPC Communication**: Type-safe communication between processes using tRPC
- **UI Components**: Demo interfaces for testing core functionality (EventsDemo, SettingsDemo, TrpcDemo)
- **EventCache**: Stores and manages recent events with filtering capabilities
- **Tailwind/ShadCN UI**: Integrated UI framework for component styling

The project has a functional reactive architecture and is ready for extension with additional service-specific adapters and more advanced event transformations.

## Dependencies and Technologies

- Electron 30.0.1 for cross-platform desktop application
- React 19.0.0 with TypeScript for UI development
- Vite with electron-vite for development and building
- electron-builder for packaging applications for different platforms
- RxJS for reactive programming and event streams
- WebSocket (ws) for external client connections
- tRPC for type-safe IPC between main and renderer processes
- RxDB for reactive, observable database with secure persistent storage
- Zod for runtime type validation
- Tailwind CSS and shadcn/ui for component styling
- @tanstack/store (in development dependencies)

## Build Commands

- `pnpm run dev` - Start the development environment
- `pnpm run start` - Preview the built application
- `pnpm run build` - Build the frontend and compile TypeScript
- `pnpm run typecheck` - Run TypeScript type checking
- `pnpm run lint` - Run ESLint for code quality
- `pnpm run format` - Run Prettier for code formatting
- `pnpm run build:mac` - Build for macOS
- `pnpm run build:win` - Build for Windows
- `pnpm run build:linux` - Build for Linux

## Environment Setup

This project requires environment variables for certain features to work properly.

### Twitch Integration

- Requires the `TWITCH_CLIENT_ID` environment variable to be set in a `.env` file
- Uses device code flow authentication (suitable for desktop applications)
- Only requests minimum necessary scopes for read operations
- See `.env.example` for reference and instructions to obtain a client ID

## Architecture

### Event-Driven Reactive Core

- **EventBus**: Central reactive system using RxJS Subjects/Observables
- **Event Streams**: Typed event streams with filtering and transformation
- **Observable Patterns**: Reactive programming for data flow and UI updates
- **Declarative Data Flow**: Transform data through Observable operators

### Authentication System

- **AuthService**: Manages authentication state and token lifecycle
- **TokenManager**: Secure storage and retrieval of authentication tokens
- **Auth Providers**: Pluggable authentication for different services
- **Device Code Flow**: Proper implementation for desktop applications
- **Token Lifecycle**: Complete management of token expiration and refresh

### Adapter System

- **BaseAdapter**: Foundation for all service adapters with common functionality
- **ServiceAdapter**: Core interface all adapters must implement
- Each adapter handles a specific platform:
  - **TwitchAdapter**: Connects to Twitch API and monitors channel/stream events
  - **ObsAdapter**: Connects to OBS via WebSockets for scene information
  - **TestAdapter**: Generates test events for debugging

### WebSocket Server

- **Real-time Events**: Streams events to external clients
- **Connection Management**: Handles client connections and disconnections
- **Standardized Format**: JSON-formatted events with consistent structure
- **Ping/Pong Protocol**: Ensures connections remain alive

### Frontend Architecture

- **Component-Based Structure**:
  - Dashboard for event monitoring
  - Settings for adapter configuration
  - Authentication UI for service connection
  - Status indicators with visual feedback
- **State Management**:
  - RxJS for reactive state management
  - Observable streams for application state
  - Clean subscription patterns with proper cleanup
- **Custom Hooks**:
  - useObservable: Connect RxJS to React components
  - useAuth: Authentication state management
  - useAdapter: Adapter control operations
  - useEventStream: Access to event data

## Code Style Guidelines

### TypeScript/React

- Use React functional components with hooks
- Integrate RxJS Observables with React using custom hooks
- Prefer explicit type annotations with TypeScript
- Use async/await for asynchronous operations
- Group related imports together (React, RxJS, internal, external)
- Follow 2-space indentation
- Split large components into smaller, focused ones
- Use RxJS patterns for state management
- Implement proper subscription cleanup in useEffect
- Design with desktop-friendly UI/UX patterns in mind

### Electron

- Keep main process code separate from renderer
- Use proper IPC patterns for main-renderer communication
- Use tRPC for type-safe IPC between main and renderer
- Implement secure preload scripts for API exposure
- Handle window management properly
- Secure token storage using Electron Store with encryption
- Manage process lifecycle correctly

### RxJS Patterns

- Treat state as Observable streams
- Use pipe() for data transformations
- Implement proper error handling in streams
- Use shareReplay() for multicasting when appropriate
- Always unsubscribe/clean up with takeUntil()
- Prefer declarative operators over imperative code
- Use BehaviorSubject for state that needs initial value
- Implement proper error recovery for streams

### Twitch Integration

- Use proper device code flow implementation for desktop apps
- Always ensure token expiration times are properly tracked and stored
- When working with tokens, make sure they're fully validated before use
- Implement automatic token refresh before expiration
- Always check the expiration of refresh tokens (30-day limit)
- When refreshing tokens, ensure the new token is properly stored

### Authentication

- Prefer OAuth device code flow for desktop applications
- Request minimal scopes necessary for functionality
- Handle token refreshing and restoration automatically
- Store tokens securely using Electron Store with encryption
- Implement proper state transitions for auth lifecycle
- Create clear user feedback during authentication process

### WebSocket Server

- Implement proper connection management
- Handle client connection/disconnection gracefully
- Use ping/pong protocol to maintain connections
- Format event messages consistently
- Ensure proper error handling for client errors
- Document the WebSocket API for consumers

### General

- Document public APIs and complex functions
- Handle errors explicitly rather than throwing
- Prefer the simplest solution that uses what the libraries provide
- Do not recreate functionality unless absolutely necessary
- Write tests for critical components
- Use ESLint and Prettier before committing

### Libraries-First Approach

- **Always check for existing libraries before writing custom code**
- Prefer well-maintained libraries with good TypeScript support and active communities
- Only build custom solutions when existing libraries don't meet specific requirements
- When using a custom solution, document why it was chosen over existing libraries

### Standard Library Stack

The following libraries are our agreed-upon standard stack for the project:

#### Core Dependencies

- **RxJS**: Foundation for reactive programming throughout the application
- **RxDB**: Reactive, observable NoSQL database for persistence
- **@tanstack/store**: Atomic state management with fine-grained reactivity
- **@tanstack/react-query**: Data fetching, caching, and synchronization
- **@tanstack/react-query-devtools**: Visual debugging for queries and cache
- **@tanstack/react-router**: Type-safe routing with first-class search params
- **@tanstack/react-table**: Headless UI for complex data tables
- **@tanstack/virtual**: Virtualized lists for optimal performance
- **tRPC**: Type-safe API calls between main and renderer processes
- **dotenv**: Environment variable management
- **zod**: Runtime validation for type safety

#### Platform Integration

- **@twurple/auth & @twurple/api**: Official Twitch API client libraries
- **obs-websocket-js**: Connectivity with OBS
- **ws**: WebSocket server implementation for external clients

#### Developer Experience

- **TypeScript**: Static typing for improved code quality
- **ESLint & Prettier**: Code formatting and linting
- **electron-vite**: Build and development setup

#### Utilities

- **date-fns**: Date manipulation (prefer over custom date handling)
- **uuid**: Generation of unique identifiers
- **clsx & tailwind-merge**: Utility for conditional class names

#### UI Components

- **tailwindcss**: Utility-first CSS framework (primary styling approach)
- **@shadcn/ui**: Reusable components built on Radix UI (preferred component library)
- **framer-motion**: Animation library (required for all animations)
- **lucide-react**: Icon library
- **tailwindcss-animate**: Animation utilities for Tailwind

**For any new functionality, first check if one of these agreed-upon libraries can handle the requirement before implementing a custom solution.**

### Logging System

- **LoggingService**: Winston-based structured logging with file and console output
- **ComponentLogger**: Context-aware loggers for services and components
- **Initialization Order**: Logging is initialized early in the startup sequence
- **Fallback Patterns**: Console loggers available during bootstrapping phase
- **Error Formatting**: Standard format for error objects with { error: error.message }
- **Log Rotation**: Automatic log rotation with 5MB files and 5 file retention
- **Log Directory**: Stored in user data folder under 'logs'

### Logging Best Practices

- Initialize component loggers in constructor: `this.logger = getLoggingService().createLogger('ComponentName')`
- Log errors with proper metadata: `logger.error('Error message', { error: error.message })`
- Include context in logs: `logger.info('Action performed', { userId, action })`
- Use appropriate log levels:
  - error: Application errors requiring attention
  - warn: Concerning situations that aren't failures
  - info: Normal operation events
  - debug: Detailed information for troubleshooting
  - trace: Very detailed debugging information
- Sensitive data should be redacted: `logger.info('User authenticated', { user: username, token: '***' })`

### Module Import Guidelines

- **Path Aliases**: Use '@m/', '@s/', '@p/' path aliases in TypeScript imports only
- **Circular Dependencies**: Avoid circular dependencies between core services
- **Initialization Order**: Be mindful of service initialization order
  1. LoggingService
  2. ConfigStore
  3. EventBus
  4. ErrorService
  5. Other services
- **Fallback Patterns**: Implement fallback behaviors when services are unavailable
- **Runtime Imports**: Avoid require() with path aliases; they may not resolve correctly
- **Cross-Process Modules**: Modules used in both main and renderer processes should use process checks

### Documentation

- Keep architecture diagrams up to date
- Document the WebSocket API clearly for external consumers
- Provide examples for common integration patterns
- Update documentation when making significant changes
