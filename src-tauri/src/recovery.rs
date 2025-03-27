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
    pub(crate) error_registry: Arc<ErrorRegistry>,
    /// Circuit breakers for different operations
    /// Now wrapped in Arc to ensure shared state across clones
    pub(crate) circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// Default retry policies for different error categories
    pub(crate) default_policies: HashMap<ErrorCategory, RetryPolicy>,
}

impl Clone for RecoveryManager {
    fn clone(&self) -> Self {
        // IMPORTANT: Maintain shared state across clones
        // Both error_registry and circuit_breakers are wrapped in Arc
        // to ensure all clones share the same state
        Self {
            error_registry: Arc::clone(&self.error_registry),
            circuit_breakers: Arc::clone(&self.circuit_breakers),
            default_policies: self.default_policies.clone(),
        }
    }
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
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
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
    // Tests for this module have been moved to src/tests/recovery_test.rs
    pub use crate::tests::recovery_test::*;
}
