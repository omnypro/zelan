// Simple WebSocket client for Zelan API
// Usage: node websocket_client.js

const WebSocket = require('ws');

// Connect to WebSocket server
const ws = new WebSocket('ws://localhost:8080');

// Connection opened
ws.on('open', function open() {
  console.log('Connected to Zelan WebSocket server');
  
  // Send client preferences to filter events (optional)
  const preferences = {
    sources: ['twitch', 'test'],            // Only events from twitch and test adapters
    event_types: ['stream.online', 'stream.offline', 'test.special']  // Only these event types
  };
  
  console.log('Sending client preferences:', preferences);
  ws.send(JSON.stringify(preferences));
});

// Listen for messages
ws.on('message', function incoming(data) {
  try {
    const event = JSON.parse(data);
    console.log('\n===== EVENT RECEIVED =====');
    console.log(`Source: ${event.source}`);
    console.log(`Type: ${event.event_type}`);
    console.log(`Time: ${event.timestamp}`);
    console.log('Payload:', JSON.stringify(event.payload, null, 2));
    console.log('=========================\n');
  } catch (error) {
    console.error('Error parsing event:', error);
    console.log('Raw data:', data);
  }
});

// Handle errors
ws.on('error', function error(err) {
  console.error('WebSocket error:', err);
});

// Handle close
ws.on('close', function close() {
  console.log('Disconnected from Zelan WebSocket server');
});

// Handle process termination
process.on('SIGINT', function() {
  console.log('Closing WebSocket connection...');
  ws.close();
  process.exit();
});

console.log('Listening for events from Zelan WebSocket server...');
console.log('Press Ctrl+C to exit');