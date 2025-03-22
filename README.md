# Zelan API Service

A containerized API service for streaming platform data aggregation. This service unifies data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API and WebSocket interface.

## Features

- **Event-Driven Architecture**: Central EventBus for real-time event distribution
- **WebSocket Interface**: Stream events in real-time to external clients
- **REST API**: Control adapters and service status via HTTP
- **Adapter System**: Pluggable service adapters for different platforms
- **Docker Support**: Easy deployment with Docker and Docker Compose

## Getting Started

### Prerequisites

- Docker and Docker Compose (for containerized deployment)
- Rust 1.76+ (for local development)
- Twitch Client ID and Client Secret (for Twitch integration)

### Environment Variables

Create a `.env` file with the following variables:

```
TWITCH_CLIENT_ID=your_twitch_client_id
TWITCH_CLIENT_SECRET=your_twitch_client_secret
API_PORT=3000
WEBSOCKET_PORT=8080
```

### Running with Docker Compose

```bash
# Build and start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
```

### Running Locally (Development)

```bash
# Install dependencies
cargo build

# Run the service
cargo run

# Run tests
cargo test
```

## API Endpoints

The service exposes the following REST API endpoints:

- **GET /api/status**: Get overall service status
- **GET /api/adapters**: Get all adapter statuses
- **GET /api/adapters/:id**: Get a specific adapter's status
- **POST /api/adapters/:id/connect**: Connect a specific adapter
- **POST /api/adapters/:id/disconnect**: Disconnect a specific adapter
- **POST /api/adapters/:id/configure**: Configure a specific adapter

### Configuration Example for Twitch Adapter

```json
POST /api/adapters/twitch/configure
{
  "config": {
    "channel_name": "your_twitch_channel",
    "polling_interval": 60
  }
}
```

### Response Format

All API responses follow this format:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

## WebSocket Interface

Connect to the WebSocket server at `ws://localhost:8080` to receive real-time events.

### Event Format

Events are sent as JSON messages with the following structure:

```json
{
  "source": "twitch",
  "event_type": "stream.online",
  "timestamp": "2023-07-04T12:34:56Z",
  "payload": { ... }
}
```

### Client Preferences

You can filter events by sending a preferences message:

```json
{
  "sources": ["twitch"],
  "event_types": ["stream.online", "stream.offline"]
}
```

## Service Adapters

The service includes the following adapters:

- **TestAdapter**: Generates test events (enabled by default)
- **TwitchAdapter**: Connects to Twitch API and monitors stream status (requires configuration)
- **ObsAdapter**: Connects to OBS via WebSockets (to be implemented)

## Event Types

### Twitch Events

- **stream.online**: When a channel goes live
- **stream.offline**: When a channel stops streaming

### Test Events

- **test.event**: Regular test events with counters
- **test.special**: Special events generated at intervals

## Project Structure

- **/src**: Source code
  - **/adapters**: Service adapter implementations
  - **/core**: Core components including EventBus
  - **/websocket**: WebSocket server implementation
  - **/api**: REST API implementation
  - **/auth**: Authentication and token management
  - **/error**: Error handling
  - **/service.rs**: Main service orchestration
- **/docker-compose.yml**: Docker Compose configuration
- **/Dockerfile**: Docker configuration

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Commit your changes: `git commit -m 'Add feature'`
4. Push to the branch: `git push origin feature-name`
5. Submit a pull request