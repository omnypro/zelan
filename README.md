# Zelan - Stream Data Aggregation Service

Zelan is a lightweight, locally-hosted data aggregation service for streaming platforms that unifies data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API.

## Features

- Reactive event-driven architecture with RxJS
- Platform-specific adapters with a common interface
- Persistent configuration using direct IPC
- Modern UI with React and Tailwind CSS
- Type safety with TypeScript
- Desktop notifications for important events

## Architecture

### Adapter System

Zelan uses an adapter pattern to integrate with different services:

- **ServiceAdapter**: Core interface all adapters must implement
- **BaseAdapter**: Common functionality shared by all adapters
- **AdapterFactory**: Factory pattern for creating adapter instances
- **AdapterManager**: Manages adapter lifecycle and configuration

### Event System

The application uses a reactive event system based on RxJS:

- **EventBus**: Central pub/sub mechanism for events
- **EventStream**: Filtered streams of events by category and type
- **IPC Bridge**: Bidirectional event communication between main and renderer processes
- **WebSocket Server**: External event streaming for overlays and tools

### WebSocket API

Zelan exposes a WebSocket API to allow external applications (overlays, tools, etc.) to receive real-time events without complex authentication:

- **Standard WebSocket**: Simple to use with any programming language or framework
- **JSON-formatted Events**: All events follow a consistent structure
- **Automatic Connection Management**: Includes ping/pong and connection cleanup
- **No Authentication Required**: Designed for local development of overlays and tools

### Configuration

Persistent configuration is managed using direct IPC communication:

- **Main Process**: Handles configuration storage and retrieval
- **IPC API**: Typed interface for configuration operations
- **React Hooks**: Custom hooks for configuration state management
- **Automatic Persistence**: Settings are automatically persisted on change

## Development

### Prerequisites

- Node.js 18+
- pnpm

### Setup

```bash
# Install dependencies
pnpm install

# Start development server
pnpm run dev

# Build production version
pnpm run build
```

### Project Structure

- `src/main` - Electron main process code
  - `adapters` - Platform-specific adapter implementations
  - `services` - Main process services
    - `adapters` - Adapter management
    - `eventBus` - Event system
    - `logging` - Structured logging system
    - `errors` - Error handling and reporting
    - `events` - Event storage and caching
    - `websocket` - WebSocket server for external clients
    - `auth` - Authentication services
- `src/renderer` - React frontend
  - `components` - UI components
  - `hooks` - React hooks for reactive state
  - `services` - Renderer process services
- `src/shared` - Shared code between main and renderer
  - `adapters` - Adapter interfaces and base classes
  - `core` - Core functionality (event bus, configuration)
  - `types` - TypeScript types and interfaces
  - `utils` - Shared utility functions
- `src/preload` - Electron preload script for IPC

#### Build Issues

If you encounter build errors:

1. Run `pnpm run typecheck` first to identify type issues
2. Check for path alias issues in tsconfig.json and electron.vite.config.ts
3. Ensure all dependencies are properly installed with `pnpm install`

## License

MIT
