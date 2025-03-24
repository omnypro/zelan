//! Integration tests for the Zelan application
//! These tests focus on testing components working together rather than individual units

// Import the test harness
pub mod test_harness;
pub mod ws_client;
pub mod ws_test_harness;

// Import individual test modules
pub mod event_bus_test;
pub mod adapter_lifecycle_test;
pub mod websocket_server_test;
pub mod callback_integrity_test;
pub mod flow_tracing_test;

// This function is called by cargo test, and is the entry point for the integration tests
#[cfg(test)]
pub fn run_tests() {
    // Any setup code before tests run could go here
    println!("Running integration tests...");
}