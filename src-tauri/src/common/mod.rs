//! Common utilities for the Zelan application.
//! 
//! This module contains shared utilities and abstractions used throughout
//! the application, focusing on concurrent safety and error handling.

pub mod shared_state;
pub mod concurrent_map;
pub mod event_registry;
#[cfg(test)]
mod tests;

// Re-export common types for easier access
pub use shared_state::{SharedState, OptionalSharedState, RefreshableState, LockError};
pub use concurrent_map::{ConcurrentMap, ConcurrentSet, MapError};
pub use event_registry::{EventRegistry, EventTypeRegistry, EventPublisher, EventSubscriber, EventRegistryError};