# Callback System

This module provides a standardized way to handle callbacks throughout the application, particularly focusing on maintaining callback integrity across clone boundaries and async contexts.

## Key Components

### `CallbackRegistry<T>`

A type-safe container for callbacks that ensures they're preserved when objects are cloned. Uses `Arc<RwLock<>>` internally to maintain shared state.

Features:
- Register callbacks with unique IDs
- Trigger all registered callbacks
- Handle errors from callbacks
- Track callback statistics

### `CallbackManager`

A central registry for managing multiple callback registries of different types.

Features:
- Get or create registries by name
- Track statistics across all registries
- Type-safe access to registries

## How to Use

### Basic Usage

```rust
use zelan_lib::callback_system::{CallbackRegistry, CallbackData};

// Define a data type for your callbacks
#[derive(Clone, Debug)]
struct MyEventData {
    event_type: String,
    payload: serde_json::Value,
}

// Implement CallbackData for your type (should be automatic)
impl CallbackData for MyEventData {}

// Create a callback registry
let registry = CallbackRegistry::<MyEventData>::new();

// Register a callback
let callback_id = registry.register(|event| {
    println!("Event received: {:?}", event);
    Ok(())
}).await;

// Trigger callbacks
let event = MyEventData {
    event_type: "test".to_string(),
    payload: serde_json::json!({ "value": 42 }),
};
let count = registry.trigger(event).await?;
```

### Advanced Usage with Adapter Cloning

```rust
use std::sync::Arc;
use zelan_lib::callback_system::CallbackRegistry;

// In your adapter struct
struct MyAdapter {
    // Use Arc to share the registry across clones
    callbacks: Arc<CallbackRegistry<MyEventData>>,
    // ... other fields
}

impl Clone for MyAdapter {
    fn clone(&self) -> Self {
        Self {
            // Clone the Arc reference, not the registry itself
            callbacks: Arc::clone(&self.callbacks),
            // ... clone other fields
        }
    }
}

impl MyAdapter {
    // Register a callback
    pub async fn register_callback<F>(&self, callback: F) -> CallbackId
    where
        F: Fn(MyEventData) -> Result<()> + Send + Sync + 'static,
    {
        self.callbacks.register(callback).await
    }
    
    // Trigger callbacks
    pub async fn trigger_event(&self, event: MyEventData) -> Result<()> {
        self.callbacks.trigger(event).await?;
        Ok(())
    }
}
```

## Benefits

1. **Clone Safety**: Callbacks are properly preserved when objects are cloned
2. **Type Safety**: Strong typing for callback data
3. **Error Handling**: Robust error collection and reporting
4. **Diagnostics**: Detailed logging for callback operations
5. **Centralization**: Consistent approach across the codebase
6. **Monitoring**: Statistics exposed via the WebSocket API (`callback.stats` command) and HTTP API (`/callbacks` endpoint)

## Implementation Details

The system uses a few key Rust patterns to ensure thread safety and proper sharing:

1. `Arc<RwLock<>>` for shared state across threads and clones
2. Generic types with trait bounds for type safety
3. `async/await` for non-blocking operations
4. Unique IDs (UUIDs) for callback tracking
5. Detailed error messages for debugging

## Best Practices

1. Always use `Arc::clone()` when cloning objects with callback registries
2. Use descriptive group names for easier debugging
3. Handle errors from trigger() properly in your code
4. Consider using CallbackManager for applications with many different event types
5. Monitor callback statistics via WebSocket API and HTTP endpoints

## Monitoring

The callback system provides monitoring endpoints to check the health and activity of registered callbacks:

### WebSocket API

Use the `callback.stats` command:

```json
{
  "command": "callback.stats"
}
```

The response includes counts of callbacks registered for each system:

```json
{
  "success": true,
  "command": "callback.stats",
  "data": {
    "stats": {
      "twitch_auth": 2,
      "obs_events": 3,
      "test_events": 1
    },
    "timestamp": "2025-03-24T12:34:56.789Z",
    "description": "Number of callbacks registered per system"
  }
}
```

### HTTP API

Access callback statistics via the HTTP API endpoint:

```
GET /callbacks
```

This returns a JSON object with counts for each registry:

```json
{
  "twitch_auth": 2,
  "obs_events": 3,
  "test_events": 1
}
```