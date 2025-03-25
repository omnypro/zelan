# Zelan WebSocket API

This document provides information about the WebSocket API exposed by the Zelan application for real-time event streaming.

## Connection

Connect to the WebSocket server at `ws://localhost:9000` (default port, configurable).

## Commands

The WebSocket API supports the following commands:

### Basic Commands

| Command | Description |
|---------|-------------|
| `ping` | Simple ping command to check if the server is alive. Server responds with `pong`. |
| `info` | Get information about available commands and event sources. |
| `callback.stats` | Get statistics about registered callbacks for various systems. |

### Subscription Commands

| Command | Description | Parameters |
|---------|-------------|------------|
| `subscribe.sources` | Subscribe to specific event sources | `data`: Array of source names, e.g., `["twitch", "obs"]` |
| `subscribe.types` | Subscribe to specific event types | `data`: Array of event types, e.g., `["stream.online", "scene.changed"]` |
| `unsubscribe.all` | Remove all filters and subscribe to all events | None |

## Command Format

Send commands as JSON objects with the following format:

```json
{
  "command": "command_name",
  "data": {} // Optional command-specific data
}
```

## Examples

### Get Information

Request:
```json
{
  "command": "info"
}
```

Response:
```json
{
  "success": true,
  "command": "info",
  "data": {
    "commands": [
      "ping",
      "info",
      "subscribe.sources",
      "subscribe.types",
      "unsubscribe.all",
      "callback.stats"
    ],
    "active_filters": {
      "filtering_active": false,
      "sources": [],
      "types": []
    },
    "event_sources": [
      "twitch",
      "obs",
      "test"
    ],
    "callback_systems": {
      "twitch_auth": "Authentication events from Twitch - auth state changes, token refresh, etc.",
      "obs_events": "OBS scene changes, stream status, connection events",
      "test_events": "Test events (standard, special, initial)"
    }
  }
}
```

### Get Callback Statistics

Request:
```json
{
  "command": "callback.stats"
}
```

Response:
```json
{
  "success": true,
  "command": "callback.stats",
  "data": {
    "stats": {
      "twitch_auth": 2,
      "obs_events": 1,
      "test_events": 0
    },
    "timestamp": "2025-03-24T12:34:56.789Z",
    "description": "Number of callbacks registered per system"
  }
}
```

### Subscribe to Specific Events

Request:
```json
{
  "command": "subscribe.sources",
  "data": ["twitch", "obs"]
}
```

Response:
```json
{
  "success": true,
  "command": "subscribe.sources",
  "message": "Subscribed to 2 sources"
}
```

## Event Format

Events are sent as JSON objects with the following format:

```json
{
  "source": "twitch",
  "event_type": "stream.online",
  "payload": {
    // Event-specific data
  },
  "timestamp": "2025-03-24T12:34:56.789Z",
  "version": 1,
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

## Available Event Sources

- `twitch`: Events from Twitch (stream status, subscriptions, follows, etc.)
- `obs`: Events from OBS (scene changes, streaming/recording status, etc.)
- `test`: Test events for development and debugging

## Notes

- The server may disconnect inactive clients after a configurable timeout period (default: 5 minutes)
- The server sends periodic pings to check if clients are still alive (default: every 60 seconds)
- The maximum number of simultaneous connections is configurable (default: 100)