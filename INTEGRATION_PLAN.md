# Streamlined Integration Plan: Zelan WebSocket Event Service

This document outlines our strategy for combining the Electron UI with the Tauri backend for Zelan, focusing on its core purpose as a middleware event delivery service.

## Phase 1: WebSocket Delivery System Enhancement ✅

1. **Standardize Event Payload Format** ✅
   - ✅ Created consistent payload schemas for all supported EventSub event types (now 13)
   - ✅ Designed versioned WebSocket message format for backward compatibility
   - ✅ Implemented efficient serialization/deserialization with caching for low latency
   - ✅ Defined clear documentation for client developers

2. **WebSocket Server Optimization** ✅
   - ✅ Enhanced WebSocket server reliability with proper error handling
   - ✅ Implemented connection monitoring and management
   - ✅ Added client subscription filtering options (by event source and type)
   - ✅ Optimized for low latency event delivery
   - ✅ Added configurable connection limits and timeout settings

3. **Create Type Definitions** ✅
   - ✅ Enhanced StreamEvent structure with version and ID fields
   - ✅ Defined comprehensive event schemas in documentation

## Phase 2: Build a Minimal Management UI

1. **Service Control Panel**
   - Simple adapter status indicators (connected/disconnected)
   - Lightweight metrics for monitoring (event count, connections)
   - Basic start/stop controls and configuration 
   - Error logging with severity indicators

2. **Configure Tauri Commands**
   - Implement essential commands for adapter control
   - Create minimal command surface for UI needs
   - Ensure proper error handling and async patterns
   - Optimize for performance and reliability

3. **Minimal React Hooks**
   - Create focused hooks (useAdapterStatus, useBasicConfig)
   - Ensure hooks maintain a simple interface for minimal components
   - Implement efficient polling or event-based updates

## Phase 3: Developer Tools

1. **WebSocket Test Client** ✅
   - ✅ Implemented a browser-based test client for monitoring events
   - ✅ Added payload inspection capabilities with JSON formatting
   - ✅ Created filtering for event types and sources
   - ✅ Added connection status and management interface
   - ✅ Implemented subscription management UI

2. **API Documentation** ✅
   - ✅ Created comprehensive API docs for WebSocket interface
   - ✅ Documented payload formats for all event types
   - ✅ Provided example code for JavaScript clients
   - ✅ Included connection and subscription examples

3. **Configuration Management**
   - Create simple UI for managing authentication
   - Implement token storage with proper security
   - Add basic adapter configuration options
   - Ensure settings persistence

## Phase 4: Testing & Performance

1. **Load Testing**
   - Test system under high event volume conditions
   - Measure and optimize latency and throughput
   - Verify memory usage remains stable over time
   - Ensure graceful degradation under stress

2. **Client Compatibility**
   - Test with various WebSocket client implementations
   - Verify payload compatibility across platforms
   - Ensure proper handling of connection edge cases
   - Create sample clients for common platforms

3. **Integration Testing**
   - Test full system workflows (authentication, event delivery)
   - Validate performance metrics
   - Test error recovery scenarios
   - Verify token refresh mechanisms

## Practical Implementation Approach

Let's break this down into concrete steps to get started:

1. **First Week: WebSocket Enhancement**
   - Standardize event payload format for all 13 event types
   - Optimize WebSocket server for reliability and performance
   - Implement connection monitoring and diagnostics

2. **Second Week: Control UI & Configuration**
   - Build minimal adapter management interface
   - Create configuration UI for WebSocket settings
   - Implement authentication flow for Twitch

3. **Third Week: Developer Tools & Testing**
   - Create built-in WebSocket test client
   - Generate API documentation for client developers
   - Implement comprehensive testing suite
   - Optimize performance based on test results

## Code Implementation Strategy

Here's a minimal approach for the WebSocket server enhancement:

1. **WebSocket Message Format:**

```rust
// In src-tauri/src/adapters/websocket_server.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WebSocketMessage {
    // Message format version for compatibility
    version: u8,
    // Event type (e.g., "twitch.stream.online")
    event_type: String,
    // Unique event ID
    id: String,
    // Timestamp in ISO format
    timestamp: String,
    // The actual event payload
    payload: serde_json::Value,
    // Source adapter identifier
    source: String,
}

impl WebSocketServer {
    // Broadcast an event to all connected clients
    pub async fn broadcast_event(&self, event_type: &str, payload: serde_json::Value, source: &str) -> Result<()> {
        let message = WebSocketMessage {
            version: 1,
            event_type: event_type.to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
            source: source.to_string(),
        };
        
        let message_json = serde_json::to_string(&message)?;
        
        let connections = self.connections.read().await;
        for connection in connections.values() {
            // Skip sending to clients who have set filters that don't match this event
            if !connection.should_receive_event(event_type) {
                continue;
            }
            
            if let Err(e) = connection.send(message_json.clone()).await {
                log::error!("Failed to send message to client: {}", e);
                // Connection failures handled separately with cleanup
            }
        }
        
        Ok(())
    }
}
```

2. **Minimal UI Component for Adapter Status:**

```typescript
// In src/components/AdapterStatus.tsx
import React from 'react';
import { useAdapterStatus } from '../hooks/useAdapterStatus';

export default function AdapterStatus() {
  const { adapters, startAdapter, stopAdapter } = useAdapterStatus();
  
  return (
    <div className="p-4 bg-gray-900 rounded-lg">
      <h2 className="text-lg font-semibold mb-4">Service Status</h2>
      
      <div className="space-y-2">
        {adapters.map(adapter => (
          <div key={adapter.id} className="flex items-center justify-between p-2 bg-gray-800 rounded">
            <div>
              <span className="font-medium">{adapter.name}</span>
              <div className="flex items-center mt-1">
                <div className={`w-2 h-2 rounded-full mr-2 ${
                  adapter.connected ? 'bg-green-500' : 'bg-red-500'
                }`} />
                <span className="text-sm text-gray-400">
                  {adapter.connected ? 'Connected' : 'Disconnected'}
                </span>
              </div>
            </div>
            
            <button
              onClick={() => adapter.connected ? stopAdapter(adapter.id) : startAdapter(adapter.id)}
              className={`px-3 py-1 rounded text-sm ${
                adapter.connected 
                  ? 'bg-red-600 hover:bg-red-700' 
                  : 'bg-blue-600 hover:bg-blue-700'
              }`}
            >
              {adapter.connected ? 'Stop' : 'Start'}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
```

3. **WebSocket Test Client Component:**

```typescript
// In src/components/WebSocketTester.tsx
import React, { useState, useEffect, useRef } from 'react';

export default function WebSocketTester() {
  const [isConnected, setIsConnected] = useState(false);
  const [messages, setMessages] = useState<any[]>([]);
  const [filter, setFilter] = useState('');
  const wsRef = useRef<WebSocket | null>(null);
  
  // Connect to the local WebSocket server
  const connect = () => {
    const ws = new WebSocket('ws://localhost:8080');
    
    ws.onopen = () => {
      setIsConnected(true);
    };
    
    ws.onclose = () => {
      setIsConnected(false);
    };
    
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        setMessages(prev => [data, ...prev].slice(0, 100)); // Keep latest 100 messages
      } catch (e) {
        console.error('Failed to parse message:', e);
      }
    };
    
    wsRef.current = ws;
  };
  
  // Disconnect from the WebSocket server
  const disconnect = () => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  };
  
  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);
  
  // Filter messages by event type
  const filteredMessages = filter
    ? messages.filter(m => m.event_type.includes(filter))
    : messages;
  
  return (
    <div className="p-4 bg-gray-900 rounded-lg">
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-lg font-semibold">WebSocket Tester</h2>
        
        <div>
          {isConnected ? (
            <button 
              onClick={disconnect}
              className="px-3 py-1 bg-red-600 hover:bg-red-700 rounded"
            >
              Disconnect
            </button>
          ) : (
            <button 
              onClick={connect}
              className="px-3 py-1 bg-blue-600 hover:bg-blue-700 rounded"
            >
              Connect
            </button>
          )}
        </div>
      </div>
      
      <div className="mb-4">
        <input
          type="text"
          value={filter}
          onChange={e => setFilter(e.target.value)}
          placeholder="Filter by event type..."
          className="w-full p-2 bg-gray-800 rounded"
        />
      </div>
      
      <div className="h-96 overflow-y-auto space-y-2">
        {filteredMessages.map((msg, i) => (
          <div key={i} className="p-2 bg-gray-800 rounded">
            <div className="flex justify-between text-sm">
              <span className="font-medium">{msg.event_type}</span>
              <span className="text-gray-400">{new Date(msg.timestamp).toLocaleTimeString()}</span>
            </div>
            <pre className="mt-1 text-xs overflow-x-auto">{JSON.stringify(msg.payload, null, 2)}</pre>
          </div>
        ))}
      </div>
    </div>
  );
}
```

## Critical Success Factors

1. **Reliability**: Ensure WebSocket connections remain stable with proper error recovery
2. **Performance**: Optimize for low latency event delivery
3. **Simplicity**: Keep the UI minimal and focused on core functionality
4. **Developer Experience**: Make it easy for client applications to consume events
5. **Security**: Ensure proper token management and authentication

## Timeline and Milestones

### Week 1 ✅
- [x] Standardize event payload format for all 13 EventSub types
- [x] Optimize WebSocket server performance and reliability
- [x] Implement connection monitoring and diagnostics

### Week 2 (Current)
- [x] Implement event filtering capabilities
- [x] Create WebSocket test client
- [x] Generate API documentation for client developers
- [ ] Build minimal adapter management interface
- [ ] Create configuration UI for WebSocket settings

### Week 3
- [ ] Implement authentication flow and token management
- [ ] Complete UI integration
- [ ] Create sample client implementations

### Week 4
- [ ] Comprehensive performance testing
- [ ] Final integrations and refinements
- [ ] User acceptance testing

## Resources Required

- **Development**: 1-2 engineers familiar with both React and Rust
- **Testing**: Test environment with access to Twitch API
- **Tools**: Tauri development environment, TypeScript tooling, WebSocket testing tools

## Risks and Mitigation

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| WebSocket reliability | High | Medium | Implement robust reconnection logic, connection monitoring |
| Event processing latency | High | Medium | Optimize serialization, minimize unnecessary processing |
| Authentication failures | High | Medium | Thorough token refresh testing, proper error handling |
| Client compatibility issues | Medium | Medium | Test with multiple client implementations, follow standards |
| EventSub API changes | Medium | Low | Version message format, implement graceful degradation |
| High event volume | High | Medium | Implement rate limiting, batching, and efficient processing |