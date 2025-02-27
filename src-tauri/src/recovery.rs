use crate::error::{
    with_retry, CircuitBreaker, ErrorCategory, ErrorRegistry, RetryPolicy, ZelanError, ZelanResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::RwLock;
use tracing::debug;

/// Recovery manager for centralizing error recovery strategies
pub struct RecoveryManager {
    /// Error registry for tracking errors
    error_registry: Arc<ErrorRegistry>,
    /// Circuit breakers for different operations
    circuit_breakers: RwLock<HashMap<String, Arc<CircuitBreaker>>>,
    /// Default retry policies for different error categories
    default_policies: HashMap<ErrorCategory, RetryPolicy>,
}

impl RecoveryManager {
    /// Create a new recovery manager with default settings
    pub fn new() -> Self {
        let error_registry = Arc::new(ErrorRegistry::new(1000));

        // Set up default retry policies
        let mut default_policies = HashMap::new();
        default_policies.insert(
            ErrorCategory::Network,
            RetryPolicy::exponential_backoff(
                5,
                Duration::from_millis(500),
                2.0,
                Some(Duration::from_secs(30)),
            ),
        );
        default_policies.insert(
            ErrorCategory::Authentication,
            RetryPolicy::fixed_delay(2, Duration::from_secs(2)),
        );
        default_policies.insert(
            ErrorCategory::RateLimit,
            RetryPolicy::exponential_backoff(
                3,
                Duration::from_secs(1),
                2.0,
                Some(Duration::from_secs(60)),
            ),
        );
        default_policies.insert(
            ErrorCategory::ServiceUnavailable,
            RetryPolicy::exponential_backoff(
                5,
                Duration::from_secs(5),
                2.0,
                Some(Duration::from_secs(120)),
            ),
        );

        Self {
            error_registry,
            circuit_breakers: RwLock::new(HashMap::new()),
            default_policies,
        }
    }

    /// Get the error registry
    pub fn error_registry(&self) -> Arc<ErrorRegistry> {
        self.error_registry.clone()
    }

    /// Register an error with the registry
    pub async fn register_error(&self, error: ZelanError, source: Option<&str>) -> ZelanError {
        self.error_registry.register(error, source).await
    }

    /// Get or create a circuit breaker for a specific operation
    pub async fn get_circuit_breaker(&self, name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;

        if let Some(breaker) = breakers.get(name) {
            return breaker.clone();
        }

        // Create a new circuit breaker with default settings
        let breaker = Arc::new(
            CircuitBreaker::new(name, 5, Duration::from_secs(60))
                .with_error_registry(self.error_registry.clone()),
        );

        breakers.insert(name.to_string(), breaker.clone());
        breaker
    }

    /// Create a circuit breaker with custom settings
    pub async fn create_circuit_breaker(
        &self,
        name: &str,
        failure_threshold: usize,
        reset_timeout: Duration,
    ) -> Arc<CircuitBreaker> {
        let breaker = Arc::new(
            CircuitBreaker::new(name, failure_threshold, reset_timeout)
                .with_error_registry(self.error_registry.clone()),
        );

        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(name.to_string(), breaker.clone());

        breaker
    }

    /// Get the default retry policy for an error category
    pub fn get_default_policy(&self, category: ErrorCategory) -> RetryPolicy {
        self.default_policies
            .get(&category)
            .cloned()
            .unwrap_or_else(|| category.default_retry_policy())
    }

    /// Execute a function with automatic retry based on error category
    pub async fn with_auto_retry<F, Fut, T>(&self, operation_name: &str, f: F) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        // First attempt without retry to get the error category
        match f().await {
            Ok(value) => Ok(value),
            Err(err) => {
                // Register the error
                let registered_err = self
                    .error_registry
                    .register(err, Some(operation_name))
                    .await;

                // Determine retry policy based on error category
                if let Some(category) = registered_err.category {
                    if category.is_retryable() {
                        let policy = self.get_default_policy(category);

                        // If retryable, use the policy to retry
                        debug!(
                            operation = operation_name,
                            category = ?category,
                            max_retries = policy.max_retries,
                            "Using automatic retry for operation"
                        );

                        return with_retry(operation_name, &policy, f).await;
                    }
                }

                // Not retryable or no category
                Err(registered_err)
            }
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn with_circuit_breaker<F, Fut, T>(
        &self,
        operation_name: &str,
        f: F,
    ) -> ZelanResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ZelanResult<T>>,
    {
        let breaker = self.get_circuit_breaker(operation_name).await;
        breaker.execute(f).await
    }

    /// Execute a function with both circuit breaker and retry
    pub async fn with_protection<F, Fut, T>(&self, operation_name: &str, f: F) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        let breaker = self.get_circuit_breaker(operation_name).await;
        let f_clone = f.clone();

        // Use the circuit breaker to protect the retry logic
        breaker
            .execute(|| async move {
                // Capture a reference to self that can be moved into the closure
                let this = self;

                // First attempt without retry to get the error category
                match f().await {
                    Ok(value) => Ok(value),
                    Err(err) => {
                        // Register the error
                        let registered_err = this
                            .error_registry
                            .register(err, Some(operation_name))
                            .await;

                        // Determine retry policy based on error category
                        if let Some(category) = registered_err.category {
                            if category.is_retryable() {
                                let policy = this.get_default_policy(category);

                                // If retryable, use the policy to retry
                                debug!(
                                    operation = operation_name,
                                    category = ?category,
                                    max_retries = policy.max_retries,
                                    "Using automatic retry with circuit breaker for operation"
                                );

                                return with_retry(operation_name, &policy, f_clone).await;
                            }
                        }

                        // Not retryable or no category
                        Err(registered_err)
                    }
                }
            })
            .await
    }
}

/// AdapterRecovery trait for adapter-specific recovery strategies
pub trait AdapterRecovery {
    /// Get the recovery manager
    fn recovery_manager(&self) -> Arc<RecoveryManager>;

    /// Execute an adapter operation with protection
    #[allow(async_fn_in_trait)]
    async fn protected_operation<F, Fut, T>(&self, operation_name: &str, f: F) -> ZelanResult<T>
    where
        F: Fn() -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = ZelanResult<T>> + Send,
    {
        self.recovery_manager()
            .with_protection(operation_name, f)
            .await
    }

    /// Register an adapter-specific error
    #[allow(async_fn_in_trait)]
    async fn register_error(&self, error: ZelanError) -> ZelanError {
        let adapter_name = self.adapter_name();
        self.recovery_manager()
            .register_error(error, Some(adapter_name))
            .await
    }

    /// Get the adapter name for error registration
    fn adapter_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorCode, ErrorSeverity, ZelanError};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

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
    async fn test_retry_policy() {
        let policy = RetryPolicy::exponential_backoff(
            3,
            Duration::from_millis(100),
            2.0,
            Some(Duration::from_secs(1)),
        );

        // First attempt should have no delay
        assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));

        // Second attempt should have base delay
        let delay = policy.calculate_delay(1);
        assert!(delay.as_millis() >= 75 && delay.as_millis() <= 150);

        // Third attempt should have doubled delay (with some jitter)
        let delay = policy.calculate_delay(2);
        assert!(delay.as_millis() >= 150 && delay.as_millis() <= 300);

        // Over max retries should have no delay
        assert_eq!(policy.calculate_delay(4), Duration::from_millis(0));
    }

    #[tokio::test]
    async fn test_auto_retry_success_after_failures() {
        let manager = RecoveryManager::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let result: ZelanResult<usize> = manager
            .with_auto_retry("test_operation", || {
                let counter_clone = counter.clone();
                async move {
                    let _current = counter_clone.fetch_add(1, Ordering::SeqCst);

                    // Succeed after 2 attempts
                    if _current < 2 {
                        Err(create_network_error())
                    } else {
                        Ok(_current)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Should be the value from the 3rd attempt
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Should have made 3 attempts
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
            circuit_breakers: RwLock::new(HashMap::new()),
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
            circuit_breakers: RwLock::new(HashMap::new()),
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
}
