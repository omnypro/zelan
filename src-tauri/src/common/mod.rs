//! Common utilities for the Zelan application.
//!
//! This module contains shared utilities and abstractions used throughout
//! the application, focusing on concurrent safety and error handling.

pub mod event_registry;
pub mod shared_state;
#[cfg(test)]
mod tests;

// Re-export common types for easier access
pub use event_registry::{
    EventPublisher, EventRegistry, EventRegistryError, EventSubscriber, EventTypeRegistry,
};
pub use shared_state::{LockError, OptionalSharedState, RefreshableState, SharedState};

// Re-export dashmap types for direct use (no custom abstractions)
pub use dashmap::{DashMap, DashSet};
