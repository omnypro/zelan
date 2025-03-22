#!/usr/bin/env python3
# Simple WebSocket client for Zelan API in Python
# Usage: python3 websocket_client.py

import asyncio
import json
import websockets
import signal
import sys

# Set to True to see all events, False to filter
SHOW_ALL_EVENTS = True

async def connect_to_zelan():
    try:
        async with websockets.connect('ws://localhost:8080') as websocket:
            print("Connected to Zelan WebSocket server")
            
            # Send client preferences to filter events if not showing all
            if not SHOW_ALL_EVENTS:
                preferences = {
                    "sources": ["twitch", "test"],  # Only events from twitch and test adapters
                    "event_types": ["stream.online", "stream.offline", "test.special"]  # Only these event types
                }
                
                print(f"Sending client preferences: {preferences}")
                await websocket.send(json.dumps(preferences))
            
            print("Listening for events...")
            
            # Listen for events
            while True:
                try:
                    data = await websocket.recv()
                    event = json.loads(data)
                    
                    print("\n===== EVENT RECEIVED =====")
                    print(f"Source: {event['source']}")
                    print(f"Type: {event['event_type']}")
                    print(f"Time: {event['timestamp']}")
                    print(f"Payload: {json.dumps(event['payload'], indent=2)}")
                    print("=========================\n")
                except json.JSONDecodeError as e:
                    print(f"Error parsing event: {e}")
                    print(f"Raw data: {data}")
    
    except websockets.exceptions.ConnectionClosed:
        print("Connection to Zelan WebSocket server closed")
    except Exception as e:
        print(f"Error connecting to Zelan WebSocket server: {e}")

async def main():
    # Setup clean shutdown
    loop = asyncio.get_event_loop()
    for s in [signal.SIGINT, signal.SIGTERM]:
        loop.add_signal_handler(s, lambda: asyncio.create_task(shutdown(loop)))
    
    print("Connecting to Zelan WebSocket server...")
    try:
        await connect_to_zelan()
    except KeyboardInterrupt:
        print("Interrupted by user, shutting down")

async def shutdown(loop):
    print("Shutting down...")
    for task in asyncio.all_tasks():
        if task is not asyncio.current_task():
            task.cancel()
    loop.stop()
    sys.exit(0)

if __name__ == "__main__":
    asyncio.run(main())