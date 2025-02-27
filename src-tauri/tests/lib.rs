// Main entry point for integration tests
pub mod integration;

// Re-export to make the tests visible to cargo test
pub use integration::*;