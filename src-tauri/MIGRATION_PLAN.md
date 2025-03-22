# Zelan Migration Plan: Desktop App to Containerized API Service

## Overview
This document outlines the plan to migrate Zelan from a Tauri desktop application to a containerized API service with the option to connect a frontend later. This approach will focus on the core functionality of providing a unified API for streaming services while simplifying the architecture.

## 1. Directory Restructuring

### Current Structure
```
/src-tauri/
  /src/
    /adapters/  - Service adapters (Twitch, OBS, etc.)
    /auth/      - Authentication
    /error.rs   - Error handling
    /lib.rs     - Core library
    /main.rs    - Tauri entry point
    /plugin.rs  - Tauri plugin
    /recovery.rs - Recovery system
```

### New Target Structure
```
/zelan/
  /src/
    /adapters/      - Service adapters (Twitch, OBS, etc.)
    /api/           - API endpoints
    /auth/          - Authentication
    /config/        - Configuration management
    /core/          - Core event system, bus, etc.
    /error.rs       - Error handling
    /websocket/     - WebSocket server
    main.rs         - Service entry point
  /docker/          - Docker configuration
  /examples/        - Example clients
  /docs/            - API documentation
  Cargo.toml
  Dockerfile
  .env.example
  docker-compose.yml
```

## 2. Tauri Dependency Removal

### Components to Replace
1. **Storage**
   - Replace `tauri_plugin_store` with standard file storage or a lightweight database
   - Implement a `ConfigManager` for handling persistent configuration

2. **Runtime**
   - Replace `tauri::async_runtime` with standard Tokio
   - Replace Tauri's JoinHandle with Tokio's

3. **UI Integration**
   - Remove all UI-specific code
   - Replace frontend communication with REST API and WebSocket

### Implementation Notes
- Use environment variables for configuration with defaults
- Add file-based configuration with hot-reload capability
- Create a clean shutdown mechanism for the service

## 3. Authentication Updates

### Current Authentication
- Device code flow for Twitch
- Secure storage via Tauri

### New Authentication
1. **OAuth Flows**
   - Implement standard authorization code flow with PKCE
   - Support client credentials flow for machine-to-machine

2. **API Keys**
   - Implement API key authentication for service consumers
   - Add key management endpoints (create, revoke, list)

3. **Security**
   - Use environment variables for secrets
   - Implement proper token refresh and validation
   - Add rate limiting for authentication endpoints

## 4. API Standardization

### REST API
- `/api/v1/adapters` - Adapter management
- `/api/v1/events` - Event history and stats
- `/api/v1/auth` - Authentication endpoints
- `/api/v1/config` - Configuration management

### WebSocket API
- Keep current WebSocket implementation with subscription model
- Add authentication for WebSocket connections
- Document WebSocket commands and events

### Documentation
- Generate OpenAPI specification
- Add example requests for each endpoint
- Document event types and schemas

## 5. Frontend Interface Planning

### API First Approach
- Design all APIs to be consumed by any frontend
- Implement CORS for web frontends
- Use consistent response formats

### Management UI Options
1. **Embedded Minimal UI**
   - Simple HTML/JS for basic management
   - Packaged with the service

2. **Separate Frontend Project**
   - Define clean interface points
   - Document API contracts

## 6. Containerization

### Docker Configuration
- Multi-stage build for smaller images
- Environment variable configuration
- Volume mounting for persistent data
- Health checks and proper signals handling

### Deployment
- Docker Compose for local development
- Documentation for deployment options (standalone, orchestrated)

## Implementation Phases

### Phase 1: Core Restructuring
- Set up new project structure
- Remove Tauri dependencies
- Implement basic storage

### Phase 2: API Development
- Create REST API endpoints
- Update WebSocket server
- Implement authentication

### Phase 3: Containerization
- Create Docker configuration
- Set up example configuration
- Create documentation

### Phase 4: Testing & Polish
- Add integration tests
- Add example clients
- Performance optimization

## Migration Strategy

### Code Reuse
- Keep adapter logic (Twitch, OBS)
- Keep event bus and WebSocket server
- Maintain core event types and interfaces

### Breaking Changes
- Authentication flow changes
- Configuration storage
- Startup/shutdown process

## Future Considerations
- GraphQL API as an alternative to REST
- gRPC for high-performance clients
- Admin dashboard as a separate project
- Metrics and monitoring integration