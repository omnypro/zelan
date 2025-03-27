//! Common utilities for the Zelan application.
//!
//! This module contains shared utilities and abstractions used throughout
//! the application, focusing on concurrent safety and error handling.

pub mod event_registry;
#[cfg(test)]
mod tests;

// Re-export common types for easier access
pub use event_registry::{
    EventPublisher, EventRegistry, EventRegistryError, EventSubscriber, EventTypeRegistry,
};

// Re-export tokio::sync::RwLock for direct use
pub use tokio::sync::RwLock;

// Re-export dashmap types for direct use
pub use dashmap::{DashMap, DashSet};
