//! WebSocket client preferences module
//!
//! This module provides functionality for managing client subscription
//! preferences for the WebSocket server.

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

use crate::core::StreamEvent;

/// WebSocket client subscription preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSocketClientPreferences {
    /// Event sources to include (empty means all)
    pub source_filters: HashSet<String>,
    /// Event types to include (empty means all)
    pub type_filters: HashSet<String>,
    /// Whether filtering is active
    pub filtering_active: bool,
}

impl WebSocketClientPreferences {
    /// Check if an event should be forwarded to this client
    pub fn should_receive_event(&self, event: &StreamEvent) -> bool {
        // If filtering is not active, forward all events
        if !self.filtering_active {
            return true;
        }

        // Check source filter
        if !self.source_filters.is_empty() && !self.source_filters.contains(event.source()) {
            return false;
        }

        // Check type filter
        if !self.type_filters.is_empty() && !self.type_filters.contains(event.event_type()) {
            return false;
        }

        true
    }

    /// Process a client subscription command
    pub fn process_subscription_command(&mut self, command: &str, value: &serde_json::Value) -> Result<String, String> {
        match command {
            "subscribe.sources" => {
                if let Some(sources) = value.as_array() {
                    self.source_filters.clear();
                    for source in sources {
                        if let Some(source_str) = source.as_str() {
                            self.source_filters.insert(source_str.to_string());
                        }
                    }
                    self.filtering_active = true;
                    Ok(format!("Subscribed to {} sources", self.source_filters.len()))
                } else {
                    Err("Invalid sources format, expected array".to_string())
                }
            },
            "subscribe.types" => {
                if let Some(types) = value.as_array() {
                    self.type_filters.clear();
                    for event_type in types {
                        if let Some(type_str) = event_type.as_str() {
                            self.type_filters.insert(type_str.to_string());
                        }
                    }
                    self.filtering_active = true;
                    Ok(format!("Subscribed to {} event types", self.type_filters.len()))
                } else {
                    Err("Invalid types format, expected array".to_string())
                }
            },
            "unsubscribe.all" => {
                self.source_filters.clear();
                self.type_filters.clear();
                self.filtering_active = false;
                Ok("Unsubscribed from all filters".to_string())
            },
            _ => Err(format!("Unknown command: {}", command))
        }
    }
}