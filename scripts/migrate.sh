#!/bin/bash
# Migration script for Zelan
# Helps migrate from Tauri desktop app to containerized service

set -e

# Ensure script is run from project root
if [ ! -f "Cargo.toml" ]; then
  echo "Error: Please run this script from the project root directory"
  exit 1
fi

echo "===== Zelan Migration Tool ====="
echo "This script will help migrate your Tauri app data to the containerized service"
echo

# Step 1: Find old configuration
echo "Step 1: Finding old configuration..."
OLD_CONFIG_DIR=""

if [ -d "$HOME/.config/zelan-tauri" ]; then
  OLD_CONFIG_DIR="$HOME/.config/zelan-tauri"
elif [ -d "$HOME/Library/Application Support/zelan-tauri" ]; then
  OLD_CONFIG_DIR="$HOME/Library/Application Support/zelan-tauri"
elif [ -d "$APPDATA/zelan-tauri" ]; then
  OLD_CONFIG_DIR="$APPDATA/zelan-tauri"
fi

if [ -z "$OLD_CONFIG_DIR" ]; then
  echo "No old configuration found. Skipping migration."
else
  echo "Found old configuration at: $OLD_CONFIG_DIR"
  
  # Create config directory if it doesn't exist
  mkdir -p ./config
  
  # Step 2: Copy and convert configuration
  echo "Step 2: Migrating configuration..."
  
  if [ -f "$OLD_CONFIG_DIR/zelan.config.json" ]; then
    echo "Found configuration file: $OLD_CONFIG_DIR/zelan.config.json"
    echo "Converting to new format..."
    
    # This is a placeholder - we will need to create a proper migration tool
    # to convert the configuration format
    cp "$OLD_CONFIG_DIR/zelan.config.json" ./config/old_config.json
    echo "Configuration backed up to ./config/old_config.json"
    echo "NOTE: Manual conversion is required. Please see docs/migration.md for details."
  fi
  
  # Step 3: Copy secure data
  if [ -f "$OLD_CONFIG_DIR/zelan.secure.json" ]; then
    echo "Found secure data file: $OLD_CONFIG_DIR/zelan.secure.json"
    echo "Backing up secure data..."
    cp "$OLD_CONFIG_DIR/zelan.secure.json" ./config/old_secure.json
    echo "Secure data backed up to ./config/old_secure.json"
    echo "NOTE: Manual extraction of tokens is required. Please see docs/migration.md for details."
  fi
fi

# Step 4: Create new configuration
echo "Step 4: Creating new configuration..."
if [ ! -f "./config/config.json" ]; then
  echo "Creating new configuration file..."
  echo '{
  "websocket": {
    "port": 9000,
    "max_connections": 100,
    "timeout_seconds": 300,
    "ping_interval": 60
  },
  "api": {
    "port": 9001,
    "cors_enabled": true,
    "cors_origins": [],
    "swagger_enabled": true
  },
  "adapters": {},
  "auth": {
    "auth_required": false,
    "jwt_expiration_seconds": 86400,
    "api_keys_enabled": true,
    "max_api_keys": 5
  }
}' > ./config/config.json
  echo "Created new configuration at ./config/config.json"
fi

echo
echo "Migration preparation complete!"
echo "Next steps:"
echo "1. Review the configuration in ./config/config.json"
echo "2. Start the service with 'cargo run' or 'docker-compose up'"
echo "3. See README.md for usage instructions"