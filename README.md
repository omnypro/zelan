# Zelan

Zelan is a unified API service for streaming platforms that aggregates data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API.

## Features

- **Unified API**: Access data from multiple streaming platforms through a single API
- **Real-time Events**: WebSocket server for real-time event streaming
- **Adapters**: Modular adapters for different services (Twitch, OBS, etc.)
- **Docker Support**: Easy deployment with Docker and Docker Compose
- **Authentication**: OAuth and API key support
- **Configuration**: Environment variables and file-based configuration

## Getting Started

### Prerequisites

- Rust 1.75+ (for development)
- Docker and Docker Compose (for deployment)
- Twitch Developer Account (for Twitch integration)

### Development Setup

1. Clone the repository
   ```bash
   git clone https://github.com/yourusername/zelan.git
   cd zelan
   ```

2. Copy the example environment file
   ```bash
   cp .env.example .env
   ```

3. Edit the .env file with your own settings
   ```bash
   # Set your Twitch credentials
   TWITCH_CLIENT_ID=your_twitch_client_id
   TWITCH_CLIENT_SECRET=your_twitch_client_secret
   ```

4. Build and run the project
   ```bash
   cargo run
   ```

### Docker Deployment

1. Clone the repository
   ```bash
   git clone https://github.com/yourusername/zelan.git
   cd zelan
   ```

2. Build and run with Docker Compose
   ```bash
   docker-compose up -d
   ```

## API Documentation

### REST API

The REST API is available at `http://localhost:9001/api/v1` with the following endpoints:

- **GET /api/v1/adapters** - List all adapters
- **GET /api/v1/adapters/:name** - Get adapter details
- **POST /api/v1/adapters/:name/connect** - Connect an adapter
- **POST /api/v1/adapters/:name/disconnect** - Disconnect an adapter
- **GET /api/v1/events** - Get event history
- **GET /api/v1/stats** - Get event bus statistics

### WebSocket API

The WebSocket server is available at `ws://localhost:9000` and supports the following commands:

- `ping` - Check connection
- `subscribe.sources` - Subscribe to specific event sources
- `subscribe.types` - Subscribe to specific event types
- `unsubscribe.all` - Unsubscribe from all filters

## Configuration

### Environment Variables

See the `.env.example` file for a list of all available environment variables.

### Configuration File

The configuration is stored in a JSON file. The default location depends on the platform:

- **Linux**: `~/.config/zelan/config.json`
- **macOS**: `~/Library/Application Support/zelan/config.json`
- **Windows**: `%APPDATA%\zelan\config.json`

You can override the location with the `ZELAN_CONFIG_PATH` environment variable.

## License

This project is licensed under the MIT License - see the LICENSE file for details.