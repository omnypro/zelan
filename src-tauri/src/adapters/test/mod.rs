//! Test adapter module

// Re-export the adapter module
pub mod adapter;
pub mod callback;

// Re-export the main adapter components for backward compatibility
pub use adapter::{TestAdapter, TestConfig};
pub use callback::{TestCallbackRegistry, TestEvent};
