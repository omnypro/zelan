# Zelan WebSocket API

This document describes how to connect to and use the Zelan WebSocket API. The WebSocket server provides a simple and standardized way for external applications (overlays, tools, etc.) to receive real-time events from Zelan without having to implement complex authentication mechanisms.

## Connection Details

- **WebSocket URL**: `ws://localhost:8081`
- **Protocol**: Standard WebSocket (ws://)
- **Port**: 8081 (default, configurable via settings)

## Connection Example

```javascript
// Browser JavaScript example
const socket = new WebSocket('ws://localhost:8081')

socket.onopen = (event) => {
  this.logger.info('Connected to Zelan WebSocket server')
}

socket.onmessage = (event) => {
  const data = JSON.parse(event.data)
  this.logger.info('Received event:', data)
}

socket.onclose = (event) => {
  this.logger.info('Disconnected from Zelan WebSocket server')
}

socket.onerror = (error) => {
  this.logger.error('WebSocket error:', error)
}
```

## Node.js Example

```javascript
const WebSocket = require('ws')
const socket = new WebSocket('ws://localhost:8081')

socket.on('open', () => {
  this.logger.info('Connected to Zelan WebSocket server')
})

socket.on('message', (data) => {
  const event = JSON.parse(data)
  this.logger.info('Received event:', event)
})

socket.on('close', () => {
  this.logger.info('Disconnected from Zelan WebSocket server')
})

socket.on('error', (error) => {
  this.logger.error('WebSocket error:', error)
})
```

## Event Format

All events sent through the WebSocket connection follow a standardized format:

```typescript
interface BaseEvent<T = unknown> {
  id: string // Unique identifier for the event
  timestamp: number // Unix timestamp when the event occurred
  source: string // Source of the event (adapter name, service, etc.)
  category: string // Event category (system, adapter, twitch, obs, etc.)
  type: string // Event type (message, subscription, follow, etc.)
  payload: T // Event-specific data
}
```

### Event Categories

The `category` field can have the following values:

- `system` - System-level events (startup, shutdown, etc.)
- `adapter` - Events related to adapters (status changes, etc.)
- `service` - Service-related events
- `twitch` - Events from Twitch
- `obs` - Events from OBS
- `user` - User-initiated events

### Example Events

#### Welcome Event

When you first connect, you'll receive a welcome event:

```json
{
  "id": "welcome",
  "timestamp": 1643673600000,
  "source": "websocket-server",
  "category": "system",
  "type": "info",
  "payload": {
    "message": "Connected to Zelan WebSocket Server",
    "clientCount": 1
  }
}
```

#### Twitch Chat Message Event

```json
{
  "id": "twitch-message-1643673600123",
  "timestamp": 1643673600123,
  "source": "twitch-adapter",
  "category": "twitch",
  "type": "message",
  "payload": {
    "channel": "channelname",
    "userId": "12345678",
    "username": "viewer123",
    "displayName": "Viewer123",
    "message": "Hello stream!",
    "isSubscriber": true,
    "isModerator": false
  }
}
```

#### OBS Scene Change Event

```json
{
  "id": "obs-scene-1643673700456",
  "timestamp": 1643673700456,
  "source": "obs-adapter",
  "category": "obs",
  "type": "scene-changed",
  "payload": {
    "sceneName": "Gameplay",
    "previousScene": "Starting Soon"
  }
}
```

## Connection Management

- The server implements a ping/pong mechanism to keep connections alive
- If your client disconnects, simply reconnect using the same URL
- The server will clean up dead connections automatically

## Error Handling

If there's a server-side error, you may receive an error event:

```json
{
  "id": "error-1643673800789",
  "timestamp": 1643673800789,
  "source": "websocket-server",
  "category": "system",
  "type": "error",
  "payload": {
    "message": "Error description"
  }
}
```

## Best Practices

1. Implement reconnection logic in case the connection drops
2. Handle JSON parsing errors gracefully
3. Filter events by category and type as needed for your application
4. Keep your WebSocket connection open rather than frequently connecting/disconnecting
5. Ensure your client responds to ping frames to maintain the connection

## Security Considerations

- The WebSocket server is only accessible from localhost by default
- No authentication is required to connect to the WebSocket server
- For remote access, consider using a secure proxy
