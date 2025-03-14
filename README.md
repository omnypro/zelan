# Zelan

A lightweight, locally-hosted data aggregation service that ingests data from streaming platforms (Twitch, OBS, etc.) and exposes it through a standardized API. Built with Rust and Tauri, Zelan enables stream overlays and third-party applications to access unified data without direct coupling to source services.

## Purpose

Zelan eliminates redundant integration work by providing a single access point for streaming-related data. Instead of implementing and debugging authentication, OAuth flows, and API interactions for each service in every project, one can connect to Zelan's local API once and access data from all supported platforms through a consistent interface.

## Key Features

- Local hosting - no dependency on cloud services like Streamlabs or StreamElements
- Unified data model across multiple streaming platforms and services
- Configurable interface for adding/managing data sources
- **Enhanced WebSocket API** for stream overlays and tools to consume aggregated data:
  - Event filtering and subscription capabilities
  - Support for 13 different Twitch EventSub event types
  - Optimized for low-latency event delivery
  - Connection management with auto-reconnect features

## Setup & Configuration

### Environment Variables

Zelan requires environment variables for certain adapters to function properly. Copy the `.env.example` file to a new file named `.env` in the project root:

```bash
cp .env.example .env
```

Then edit the `.env` file to set the required values:

#### Twitch Integration

To use the Twitch adapter, you need to obtain a Client ID:

1. Register a new application at the [Twitch Developer Console](https://dev.twitch.tv/console/apps)
2. Set the OAuth Redirect URL to `http://localhost`
3. Set your application's Category to "Other"
4. After creating the application, copy the Client ID (not the Client Secret) to your `.env` file:

```
TWITCH_CLIENT_ID=your_client_id_here
```

Note: This project uses device code flow authentication, which is designed for desktop applications that cannot securely store client secrets.

## WebSocket API

Zelan provides a WebSocket API for real-time event streaming. The WebSocket server listens on port 8080 by default (configurable in the application settings).

### Connecting to the WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8080');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(`Received ${data.event_type} from ${data.source}`);
  console.log('Payload:', data.payload);
};
```

### Event Filtering

Clients can filter which events they receive using subscription commands:

```javascript
// Subscribe to only Twitch stream events
ws.send(JSON.stringify({
  command: 'subscribe.types',
  data: ['stream.online', 'stream.offline']
}));

// Subscribe to only events from specific sources
ws.send(JSON.stringify({
  command: 'subscribe.sources',
  data: ['twitch']
}));
```

### Test Client

Zelan includes a browser-based WebSocket test client to help you explore the API and test your integrations. You can access it by opening `websocket-test-client.html` in your browser.

For complete documentation of the WebSocket API, see [WebSocket API Documentation](docs/websocket-api.md).
