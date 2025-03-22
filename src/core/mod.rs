//! Core service implementation
//!
//! This module contains the central components of the Zelan service,
//! including the event bus and stream service.

mod event_bus;
mod service;

// Export main components
pub use event_bus::{EventBus, EventBusStats, StreamEvent};
pub use service::StreamService;

// Re-export constants for default values
pub const EVENT_BUS_CAPACITY: usize = 1000;
pub const DEFAULT_WS_PORT: u16 = 9000;

/// Default max connections
pub fn default_max_connections() -> usize {
    100
}

/// Default timeout in seconds
pub fn default_timeout() -> u64 {
    300
}

/// Default ping interval in seconds
pub fn default_ping_interval() -> u64 {
    60
}