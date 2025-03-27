use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::error::{
    ErrorCategory, ErrorCode, ErrorRegistry, ErrorSeverity, RetryPolicy, ZelanError, ZelanResult,
};
use crate::recovery::RecoveryManager;

// Helper function to create a network error
fn create_network_error() -> ZelanError {
    ZelanError {
        code: ErrorCode::NetworkTimeout,
        message: "Test network timeout".to_string(),
        context: None,
        severity: ErrorSeverity::Warning,
        category: Some(ErrorCategory::Network),
        error_id: None,
    }
}

// Helper function to create a non-retryable error
fn create_non_retryable_error() -> ZelanError {
    ZelanError {
        code: ErrorCode::ConfigInvalid,
        message: "Test config error".to_string(),
        context: None,
        severity: ErrorSeverity::Error,
        category: Some(ErrorCategory::Configuration),
        error_id: None,
    }
}

#[tokio::test]
async fn test_auto_retry_gives_up_after_max_retries() {
    let _unused_manager = RecoveryManager::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // Create a retry policy with only 2 retries
    let policy = RetryPolicy::exponential_backoff(2, Duration::from_millis(10), 1.0, None);

    // Create a new manager with custom policies
    let mut custom_policies = HashMap::new();
    custom_policies.insert(ErrorCategory::Network, policy);

    // Create a new manager with these policies
    let manager = RecoveryManager {
        error_registry: Arc::new(ErrorRegistry::new(1000)),
        circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        default_policies: custom_policies,
    };

    let result: ZelanResult<usize> = manager
        .with_auto_retry("test_operation", || {
            let counter_clone = counter.clone();
            async move {
                let _current = counter_clone.fetch_add(1, Ordering::SeqCst);

                // Always fail
                Err(create_network_error())
            }
        })
        .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 4); // 1 initial + 3 retries
}

#[tokio::test]
async fn test_non_retryable_error_doesnt_retry() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Create a new manager
    let manager = RecoveryManager::new();

    let result: ZelanResult<usize> = manager
        .with_auto_retry("test_operation", || {
            let counter_clone = counter.clone();
            async move {
                let _current = counter_clone.fetch_add(1, Ordering::SeqCst);

                // Return a non-retryable error
                Err(create_non_retryable_error())
            }
        })
        .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1); // Should not retry
}

#[tokio::test]
async fn test_circuit_breaker_opens_after_failures() {
    let manager = RecoveryManager::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // Create a circuit breaker with a low threshold
    let breaker = manager
        .create_circuit_breaker("test_breaker", 2, Duration::from_secs(1))
        .await;

    // Make 3 calls that should fail and open the circuit
    for _ in 0..3 {
        let result: ZelanResult<usize> = breaker
            .execute(|| {
                let counter_clone = counter.clone();
                async move {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                    Err(create_network_error())
                }
            })
            .await;

        assert!(result.is_err());
    }

    // The circuit should be open now, so the counter shouldn't increment on next call
    let result = breaker
        .execute(|| {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 2); // Should not have incremented

    // Wait for the circuit to reset
    sleep(Duration::from_secs(1)).await;

    // Now it should work again
    let result = breaker
        .execute(|| {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(counter.load(Ordering::SeqCst), 3); // Should have incremented
}

#[tokio::test]
async fn test_combined_protection() {
    let _manager = RecoveryManager::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // Create a new manager with custom policies
    let mut custom_policies = HashMap::new();
    custom_policies.insert(
        ErrorCategory::Network,
        RetryPolicy::exponential_backoff(2, Duration::from_millis(10), 1.0, None),
    );

    // Create a new manager with these policies
    let manager = RecoveryManager {
        error_registry: Arc::new(ErrorRegistry::new(1000)),
        circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        default_policies: custom_policies,
    };

    // Create a circuit breaker with a low threshold
    let _ = manager
        .create_circuit_breaker("test_combined", 1, Duration::from_secs(1))
        .await;

    let result: ZelanResult<usize> = manager
        .with_protection("test_combined", || {
            let counter_clone = counter.clone();
            async move {
                let _current = counter_clone.fetch_add(1, Ordering::SeqCst);

                // Always fail with a retryable error
                Err(create_network_error())
            }
        })
        .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 4); // 1 initial + 3 retries

    // Make enough calls to open the circuit
    for _ in 0..3 {
        let _: ZelanResult<usize> = manager
            .with_protection("test_combined", || {
                let counter_clone = counter.clone();
                async move {
                    let _current = counter_clone.fetch_add(1, Ordering::SeqCst);
                    Err(create_network_error())
                }
            })
            .await;
    }

    // The circuit should be open now
    let result = manager
        .with_protection("test_combined", || {
            let counter_clone = counter.clone();
            async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        })
        .await;

    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 4); // Should not have incremented after circuit is open
}