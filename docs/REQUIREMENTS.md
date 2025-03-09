⏺ Zelan Project Requirements

Core Purpose

- Lightweight, locally-hosted data aggregation service for streaming platforms
- Standardized API for stream overlays and third-party applications
- Data ingestion from multiple sources (Twitch, OBS, etc.)
- Decoupled architecture for platform independence

Technical Stack

- Electron with React/TypeScript frontend
- RxJS for reactive programming
- WebSocket server for external clients
- tRPC for type-safe IPC between processes
- Winston for structured logging and diagnostics
- Tailwind CSS and shadcn/ui for styling
- Twurple libraries for Twitch integration
- obs-websocket-js for OBS connectivity

Core Systems

1. Event System: RxJS-based with EventBus and filtered streams
2. Adapter System: Platform-specific adapters with common interface
3. Authentication: Service token management with OAuth flows
4. WebSocket Server: Real-time event streaming to clients
5. Configuration: Settings and preference management
6. Logging System: Structured logging with file and console outputs
7. IPC Layer: Type-safe communication between processes

Architecture Requirements

- Process isolation (main vs renderer)
- Reactive data flow with Observable patterns
- Secure token storage
- External client API via WebSockets
- Type-safety throughout codebase
- Proper Electron security model adherence

Critical Features

- Multi-service authentication
- Event transformation and filtering
- Real-time data streaming
- Persistent configuration
- Platform-specific adapters
- Desktop notifications
