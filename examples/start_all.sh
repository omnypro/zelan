#!/bin/bash
# Start Zelan API Service and a WebSocket client

# Check if Python 3 is available
if command -v python3 &>/dev/null; then
  PYTHON_CMD="python3"
elif command -v python &>/dev/null; then
  PYTHON_CMD="python"
else
  echo "Python not found. Please install Python 3."
  exit 1
fi

# Check if we're in the examples directory or the project root
if [[ $(basename "$PWD") == "examples" ]]; then
  cd ..
fi

# Check if .env exists, if not copy from example
if [ ! -f .env ]; then
  if [ -f .env.example ]; then
    echo "Creating .env file from .env.example..."
    cp .env.example .env
    echo "Please edit .env file with your Twitch credentials before starting the service."
    exit 1
  fi
fi

# Create data directory if it doesn't exist
mkdir -p data

# Build the API service if needed
echo "Building Zelan API service..."
cargo build

# Start the API service in the background
echo "Starting Zelan API service..."
cargo run &
API_PID=$!

# Wait for the API service to start
echo "Waiting for API service to start..."
sleep 5

# Configure and connect Twitch adapter
echo "Configuring Twitch adapter..."
./examples/twitch_example.sh

# Start WebSocket client
echo "Starting WebSocket client..."
$PYTHON_CMD ./examples/websocket_client.py

# Cleanup when the script is interrupted
trap "kill $API_PID; echo 'Shutting down...'; exit" INT TERM
wait