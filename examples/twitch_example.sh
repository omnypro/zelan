#!/bin/bash
# Example script to configure and connect the Twitch adapter

# Set base URL
API_URL="http://localhost:3000/api"

# Set your Twitch channel name
TWITCH_CHANNEL="avalonstar"  # Replace with your channel name

# 1. Get all adapters and their status
echo "Getting all adapters..."
curl -s "$API_URL/adapters" | jq .

# 2. Get Twitch adapter status specifically
echo -e "\nGetting Twitch adapter status..."
curl -s "$API_URL/adapters/twitch" | jq .

# 3. Configure the Twitch adapter with your channel
echo -e "\nConfiguring Twitch adapter for channel: $TWITCH_CHANNEL..."
curl -s -X POST "$API_URL/adapters/twitch/configure" \
  -H "Content-Type: application/json" \
  -d "{\"config\": {\"channel_name\": \"$TWITCH_CHANNEL\", \"polling_interval\": 60}}" | jq .

# 4. Connect the Twitch adapter
echo -e "\nConnecting Twitch adapter..."
curl -s -X POST "$API_URL/adapters/twitch/connect" | jq .

# 5. Get Twitch adapter status after connection
echo -e "\nGetting Twitch adapter status after connection..."
curl -s "$API_URL/adapters/twitch" | jq .

echo -e "\nSetup complete! The Twitch adapter is now monitoring the $TWITCH_CHANNEL channel."
echo "Any stream online/offline events will be sent through the WebSocket server on port 8080."
echo -e "\nTo disconnect the adapter, run:"
echo "curl -X POST $API_URL/adapters/twitch/disconnect"