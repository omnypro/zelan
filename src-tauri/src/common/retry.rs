use std::fmt::Display;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Simple helper for retrying operations with configurable retry logic
/// This function provides a standardized way to implement retries without complex abstractions
/// Uses functional programming approach with recursion instead of a loop
pub async fn with_retry<T, E, F>(
    operation: F,
    max_attempts: usize,
    log_context: &str,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
    E: Display + Send,
    T: Send,
{
    // Inner recursive function for retry logic
    async fn try_operation<T, E, F>(
        operation: &F,
        attempt: usize,
        max_attempts: usize,
        log_context: &str,
    ) -> Result<T, E>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
        E: Display + Send,
        T: Send,
    {
        // Try the operation
        match operation().await {
            // Success case - log if retried and return result
            Ok(value) => {
                if attempt > 1 {
                    debug!("{} succeeded after {} attempts", log_context, attempt);
                }
                Ok(value)
            }
            
            // Error case - either retry or return error
            Err(e) => {
                // If we've reached max attempts, log and return error
                if attempt >= max_attempts {
                    error!("{} failed after {} attempts: {}", log_context, attempt, e);
                    Err(e)
                } else {
                    // Calculate exponential backoff with a cap
                    let delay = std::cmp::min(100 * 2u64.pow(attempt as u32), 5000);
                    warn!(
                        "{} failed (attempt {}/{}): {}",
                        log_context, attempt, max_attempts, e
                    );
                    
                    // Wait for backoff period
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    
                    // Recursive call for next attempt
                    try_operation(operation, attempt + 1, max_attempts, log_context).await
                }
            }
        }
    }
    
    // Start with first attempt
    try_operation(&operation, 1, max_attempts, log_context).await
}

/// Retry with a custom backoff strategy
/// Uses functional programming approach with recursion instead of a loop
pub async fn with_retry_and_backoff<T, E, F, B>(
    operation: F,
    max_attempts: usize,
    log_context: &str,
    backoff_fn: B,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
    E: Display + Send,
    B: Fn(usize) -> Duration + Send + Sync,
    T: Send,
{
    // Inner recursive function for retry logic
    async fn try_operation_with_backoff<T, E, F, B>(
        operation: &F,
        backoff_fn: &B,
        attempt: usize,
        max_attempts: usize,
        log_context: &str,
    ) -> Result<T, E>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>> + Send + Sync,
        E: Display + Send,
        B: Fn(usize) -> Duration + Send + Sync,
        T: Send,
    {
        match operation().await {
            // Success case - log if retried and return result
            Ok(value) => {
                if attempt > 1 {
                    debug!("{} succeeded after {} attempts", log_context, attempt);
                }
                Ok(value)
            }
            
            // Error case - either retry or return error
            Err(e) => {
                // If we've reached max attempts, log and return error
                if attempt >= max_attempts {
                    error!("{} failed after {} attempts: {}", log_context, attempt, e);
                    Err(e)
                } else {
                    // Use the provided backoff function
                    let delay = backoff_fn(attempt);
                    warn!(
                        "{} failed (attempt {}/{}): {}. Retrying in {:?}",
                        log_context, attempt, max_attempts, e, delay
                    );
                    
                    // Wait for backoff period
                    tokio::time::sleep(delay).await;
                    
                    // Recursive call for next attempt
                    try_operation_with_backoff(operation, backoff_fn, attempt + 1, max_attempts, log_context).await
                }
            }
        }
    }
    
    // Start with first attempt
    try_operation_with_backoff(&operation, &backoff_fn, 1, max_attempts, log_context).await
}

/// Helper to create a constant backoff duration
pub fn constant_backoff(duration_ms: u64) -> impl Fn(usize) -> Duration + Send + Sync {
    move |_| Duration::from_millis(duration_ms)
}

/// Helper to create a linear backoff strategy: base * attempt
pub fn linear_backoff(base_ms: u64) -> impl Fn(usize) -> Duration + Send + Sync {
    move |attempt| Duration::from_millis(base_ms * attempt as u64)
}

/// Helper to create an exponential backoff strategy: base * 2^(attempt-1)
/// with an optional maximum delay
pub fn exponential_backoff(
    base_ms: u64,
    max_ms: Option<u64>,
) -> impl Fn(usize) -> Duration + Send + Sync {
    move |attempt| {
        let delay = base_ms * 2u64.pow((attempt - 1) as u32);
        if let Some(max) = max_ms {
            Duration::from_millis(std::cmp::min(delay, max))
        } else {
            Duration::from_millis(delay)
        }
    }
}

/// Helper to add jitter to any backoff function
pub fn with_jitter<F>(backoff_fn: F) -> impl Fn(usize) -> Duration + Send + Sync
where
    F: Fn(usize) -> Duration + Send + Sync,
{
    move |attempt| {
        let delay = backoff_fn(attempt);
        // Add up to 25% jitter to prevent thundering herd
        let jitter_factor = fastrand::f64() * 0.25;
        let jitter_ms = (delay.as_millis() as f64 * jitter_factor) as u64;
        delay + Duration::from_millis(jitter_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_basic_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            || {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 3 {
                        Err(format!("Failed attempt {}", attempt))
                    } else {
                        Ok(format!("Success on attempt {}", attempt))
                    }
                })
            },
            5,
            "test operation",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2); // 0-indexed, so 2 means 3 attempts
    }

    #[tokio::test]
    async fn test_max_attempts() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result: Result<String, String> = with_retry(
            || {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    Err(format!("Failed attempt {}", attempt))
                })
            },
            3,
            "failing operation",
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 3 attempts total
        assert_eq!(result.unwrap_err(), "Failed attempt 3");
    }

    #[tokio::test]
    async fn test_custom_backoff() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Create a custom backoff that's very fast for testing
        let result = with_retry_and_backoff(
            || {
                let counter = counter_clone.clone();
                Box::pin(async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                    if attempt < 3 {
                        Err(format!("Failed attempt {}", attempt))
                    } else {
                        Ok(format!("Success on attempt {}", attempt))
                    }
                })
            },
            5,
            "test with custom backoff",
            constant_backoff(1), // 1ms delay for faster tests
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_backoff_strategies() {
        // Test constant backoff
        let constant = constant_backoff(100);
        assert_eq!(constant(1), Duration::from_millis(100));
        assert_eq!(constant(5), Duration::from_millis(100));

        // Test linear backoff
        let linear = linear_backoff(50);
        assert_eq!(linear(1), Duration::from_millis(50));
        assert_eq!(linear(3), Duration::from_millis(150));

        // Test exponential backoff
        let exp = exponential_backoff(50, None);
        assert_eq!(exp(1), Duration::from_millis(50));
        assert_eq!(exp(2), Duration::from_millis(100));
        assert_eq!(exp(3), Duration::from_millis(200));

        // Test exponential backoff with cap
        let exp_capped = exponential_backoff(50, Some(150));
        assert_eq!(exp_capped(1), Duration::from_millis(50));
        assert_eq!(exp_capped(2), Duration::from_millis(100));
        assert_eq!(exp_capped(3), Duration::from_millis(150)); // Capped
    }
}
