//! Common utilities for the Zelan application.
//!
//! This module contains shared utilities and abstractions used throughout
//! the application, focusing on concurrent safety and error handling.

pub mod concurrent_map;
pub mod event_registry;
pub mod shared_state;
#[cfg(test)]
mod tests;

// Re-export common types for easier access
pub use concurrent_map::{ConcurrentMap, ConcurrentSet, MapError};
pub use event_registry::{
    EventPublisher, EventRegistry, EventRegistryError, EventSubscriber, EventTypeRegistry,
};
pub use shared_state::{LockError, OptionalSharedState, RefreshableState, SharedState};
