use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use tauri::async_runtime::RwLock;
use tokio::time::timeout;
use thiserror::Error;
use tracing::{debug, error, trace, warn};

/// Errors that can occur during SharedState operations
#[derive(Error, Debug, Clone)]
pub enum LockError {
    /// The lock operation timed out
    #[error("Lock operation timed out after {duration:?}: {context}")]
    Timeout {
        /// Duration after which the operation timed out
        duration: Duration,
        /// Additional context about what was being locked
        context: String,
    },

    /// Error during read lock operation
    #[error("Failed to acquire read lock: {message}")]
    ReadFailed {
        /// Error message
        message: String,
        /// Additional context
        context: Option<String>,
    },

    /// Error during write lock operation
    #[error("Failed to acquire write lock: {message}")]
    WriteFailed {
        /// Error message
        message: String,
        /// Additional context
        context: Option<String>,
    },

    /// Attempted to access a value that wasn't initialized
    #[error("Cannot access uninitialized value")]
    Uninitialized,

    /// Error propagated from a closure or function operating on a SharedState
    #[error("Operation on shared state failed: {message}")]
    OperationFailed {
        /// Error message
        message: String,
        /// Additional context
        context: Option<String>,
        /// Whether this error is critical
        critical: bool,
    },
}

impl LockError {
    /// Create a new timeout error
    pub fn timeout(duration: Duration, context: impl Into<String>) -> Self {
        LockError::Timeout {
            duration,
            context: context.into(),
        }
    }

    /// Create a new read failed error
    pub fn read_failed(message: impl Into<String>) -> Self {
        LockError::ReadFailed {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new read failed error with context
    pub fn read_failed_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        LockError::ReadFailed {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a new write failed error
    pub fn write_failed(message: impl Into<String>) -> Self {
        LockError::WriteFailed {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new write failed error with context
    pub fn write_failed_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        LockError::WriteFailed {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a new operation failed error
    pub fn operation_failed(message: impl Into<String>) -> Self {
        LockError::OperationFailed {
            message: message.into(),
            context: None,
            critical: false,
        }
    }

    /// Create a new operation failed error with context
    pub fn operation_failed_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        LockError::OperationFailed {
            message: message.into(),
            context: Some(context.into()),
            critical: false,
        }
    }

    /// Create a new critical operation failed error
    pub fn critical_operation_failed(message: impl Into<String>) -> Self {
        LockError::OperationFailed {
            message: message.into(),
            context: None,
            critical: true,
        }
    }

    /// Returns true if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, LockError::Timeout { .. })
    }

    /// Returns true if this is a critical error
    pub fn is_critical(&self) -> bool {
        match self {
            LockError::OperationFailed { critical, .. } => *critical,
            LockError::Uninitialized => true, // Uninitialized is always critical
            _ => false,
        }
    }
}

/// A thread-safe wrapper around shared state with improved error handling
/// and timeout capabilities for read/write operations
#[derive(Clone)]
pub struct SharedState<T>
where
    T: Send + Sync + 'static
{
    /// The underlying shared state
    inner: Arc<RwLock<T>>,
    /// Debug name for this shared state (used in logging)
    name: String,
    /// Default timeout for operations
    default_timeout: Duration,
}

impl<T> SharedState<T>
where
    T: Send + Sync + 'static
{
    /// Create a new SharedState with the given initial value
    pub fn new(value: T) -> Self {
        Self::with_name_and_timeout(value, "unnamed", Duration::from_secs(30))
    }
    
    /// Create a new SharedState with the given name
    pub fn with_name(value: T, name: impl Into<String>) -> Self {
        Self::with_name_and_timeout(value, name, Duration::from_secs(30))
    }
    
    /// Create a new SharedState with the given timeout
    pub fn with_timeout(value: T, timeout_duration: Duration) -> Self {
        Self::with_name_and_timeout(value, "unnamed", timeout_duration)
    }
    
    /// Create a new SharedState with the given name and timeout
    pub fn with_name_and_timeout(value: T, name: impl Into<String>, default_timeout: Duration) -> Self {
        let name = name.into();
        debug!(name = %name, "Creating new SharedState");
        
        Self {
            inner: Arc::new(RwLock::new(value)),
            name,
            default_timeout,
        }
    }
    
    /// Get the name of this SharedState
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the default timeout for operations
    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }
    
    /// Set the default timeout for operations
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }
    
    /// Get read access to the inner value with the default timeout
    pub async fn read(&self) -> Result<T, LockError>
    where
        T: Clone,
    {
        self.read_with_timeout(self.default_timeout).await
    }
    
    /// Get read access to the inner value with a specific timeout
    pub async fn read_with_timeout(&self, timeout_duration: Duration) -> Result<T, LockError>
    where
        T: Clone,
    {
        trace!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Acquiring read lock");
        
        match timeout(timeout_duration, self.inner.read()).await {
            Ok(guard) => {
                trace!(name = %self.name, "Read lock acquired");
                let value = guard.clone();
                Ok(value)
            },
            Err(_) => {
                error!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Timeout acquiring read lock");
                Err(LockError::timeout(timeout_duration, format!("read lock on {}", self.name)))
            }
        }
    }
    
    /// Read the value, apply a function to produce a result, and return the result
    pub async fn with_read<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
    {
        self.with_read_timeout(self.default_timeout, f).await
    }
    
    /// Read the value with a specific timeout, apply a function to produce a result, and return the result
    pub async fn with_read_timeout<F, R>(&self, timeout_duration: Duration, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
    {
        trace!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Acquiring read lock for function");
        
        match timeout(timeout_duration, self.inner.read()).await {
            Ok(guard) => {
                trace!(name = %self.name, "Read lock acquired for function");
                let result = f(&guard);
                Ok(result)
            },
            Err(_) => {
                error!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Timeout acquiring read lock for function");
                Err(LockError::timeout(timeout_duration, format!("read lock on {}", self.name)))
            }
        }
    }
    
    /// Write to the value, apply a function to it, and return a result
    pub async fn with_write<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.with_write_timeout(self.default_timeout, f).await
    }
    
    /// Write to the value with a specific timeout, apply a function to it, and return a result
    pub async fn with_write_timeout<F, R>(&self, timeout_duration: Duration, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&mut T) -> R,
    {
        trace!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Acquiring write lock for function");
        
        match timeout(timeout_duration, self.inner.write()).await {
            Ok(mut guard) => {
                trace!(name = %self.name, "Write lock acquired for function");
                let result = f(&mut guard);
                Ok(result)
            },
            Err(_) => {
                error!(name = %self.name, timeout_ms = ?timeout_duration.as_millis(), "Timeout acquiring write lock for function");
                Err(LockError::timeout(timeout_duration, format!("write lock on {}", self.name)))
            }
        }
    }
    
    /// Read the value, apply a function that might return an error
    pub async fn try_with_read<F, R, E>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> Result<R, E>,
        E: fmt::Display,
    {
        match self.with_read(|value| f(value)).await? {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!(name = %self.name, error = %e, "Operation failed during read access");
                Err(LockError::operation_failed(e.to_string()))
            }
        }
    }
    
    /// Write to the value, apply a function that might return an error
    pub async fn try_with_write<F, R, E>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&mut T) -> Result<R, E>,
        E: fmt::Display,
    {
        match self.with_write(|value| f(value)).await? {
            Ok(result) => Ok(result),
            Err(e) => {
                warn!(name = %self.name, error = %e, "Operation failed during write access");
                Err(LockError::operation_failed(e.to_string()))
            }
        }
    }
    
    /// Get a clone of the inner value
    pub async fn get_cloned(&self) -> Result<T, LockError>
    where
        T: Clone,
    {
        self.read().await
    }
    
    /// Set the inner value
    pub async fn set(&self, value: T) -> Result<(), LockError> {
        self.with_write(|inner| *inner = value).await
    }
    
    /// Update the inner value using a function
    pub async fn update<F>(&self, f: F) -> Result<(), LockError>
    where
        F: FnOnce(&mut T),
    {
        self.with_write(|inner| f(inner)).await
    }
    
    /// Takes the inner value, replacing it with a default value
    pub async fn take(&self) -> Result<T, LockError>
    where
        T: Default,
    {
        self.with_write(|inner| std::mem::take(inner)).await
    }
    
    /// Replace the inner value, returning the old value
    pub async fn replace(&self, value: T) -> Result<T, LockError> {
        self.with_write(|inner| std::mem::replace(inner, value)).await
    }
}

// Implement Debug for SharedState
impl<T> fmt::Debug for SharedState<T>
where
    T: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedState")
            .field("name", &self.name)
            .field("default_timeout", &format!("{:?}", self.default_timeout))
            .field("inner", &format!("Arc<RwLock<{}>>", std::any::type_name::<T>()))
            .finish()
    }
}

/// A type for optional shared state that might not be initialized yet
#[derive(Clone, Debug)]
pub struct OptionalSharedState<T>
where
    T: Send + Sync + 'static,
{
    /// The underlying shared state
    inner: SharedState<Option<T>>,
}

impl<T> OptionalSharedState<T>
where
    T: Send + Sync + 'static,
{
    /// Create a new uninitialized OptionalSharedState
    pub fn new() -> Self {
        Self {
            inner: SharedState::with_name(None, "optional")
        }
    }
    
    /// Create a new OptionalSharedState with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            inner: SharedState::with_name(None, name)
        }
    }
    
    /// Create a new initialized OptionalSharedState
    pub fn with_value(value: T) -> Self {
        Self {
            inner: SharedState::with_name(Some(value), "optional")
        }
    }
    
    /// Create a new initialized OptionalSharedState with a name
    pub fn with_name_and_value(name: impl Into<String>, value: T) -> Self {
        Self {
            inner: SharedState::with_name(Some(value), name)
        }
    }
    
    /// Check if the state is initialized
    pub async fn is_initialized(&self) -> Result<bool, LockError> {
        self.inner.with_read(|opt| opt.is_some()).await
    }
    
    /// Read the value, apply a function to produce a result, and return the result
    /// Returns an error if the state is not initialized
    pub async fn with_read<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
    {
        self.inner.with_read(|opt| {
            match opt {
                Some(value) => Ok(f(value)),
                None => Err(LockError::Uninitialized),
            }
        }).await?
    }
    
    /// Write to the value, apply a function to it, and return a result
    /// Returns an error if the state is not initialized
    pub async fn with_write<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.inner.with_write(|opt| {
            match opt {
                Some(value) => Ok(f(value)),
                None => Err(LockError::Uninitialized),
            }
        }).await?
    }
    
    /// Initialize the state with a value
    pub async fn initialize(&self, value: T) -> Result<(), LockError> {
        self.inner.with_write(|opt| {
            *opt = Some(value);
        }).await
    }
    
    /// Initialize the state with a value if it's not already initialized
    pub async fn initialize_if_empty(&self, value: T) -> Result<bool, LockError> {
        self.inner.with_write(|opt| {
            if opt.is_none() {
                *opt = Some(value);
                true
            } else {
                false
            }
        }).await
    }
    
    /// Get a clone of the inner value
    /// Returns an error if the state is not initialized
    pub async fn get_cloned(&self) -> Result<T, LockError>
    where
        T: Clone,
    {
        self.with_read(|value| value.clone()).await
    }
    
    /// Clear the state
    pub async fn clear(&self) -> Result<(), LockError> {
        self.inner.with_write(|opt| {
            *opt = None;
        }).await
    }
    
    /// Take the value, leaving the state uninitialized
    /// Returns an error if the state is not initialized
    pub async fn take(&self) -> Result<T, LockError> {
        self.inner.with_write(|opt| {
            opt.take().ok_or(LockError::Uninitialized)
        }).await?
    }
}

/// A container for a value that can be refreshed or recreated
#[derive(Clone)]
pub struct RefreshableState<T> 
where
    T: Send + Sync + 'static,
{
    /// The underlying shared state
    inner: SharedState<T>,
    /// The last time this value was refreshed
    last_refreshed: SharedState<Option<std::time::Instant>>,
    /// Factory function to create a new value
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    /// Maximum age before the value should be refreshed
    max_age: Duration,
}

impl<T> RefreshableState<T>
where
    T: Send + Sync + 'static,
{
    /// Create a new RefreshableState with the given initial value and factory function
    pub fn new(initial_value: T, factory: impl Fn() -> T + Send + Sync + 'static, max_age: Duration) -> Self {
        Self::with_name(initial_value, "refreshable", factory, max_age)
    }
    
    /// Create a new RefreshableState with the given name
    pub fn with_name(
        initial_value: T, 
        name: impl Into<String>, 
        factory: impl Fn() -> T + Send + Sync + 'static, 
        max_age: Duration
    ) -> Self {
        let name = name.into();
        let inner = SharedState::with_name(initial_value, &name);
        let last_refreshed = SharedState::with_name(Some(std::time::Instant::now()), format!("{}_last_refreshed", name));
        
        Self {
            inner,
            last_refreshed,
            factory: Arc::new(factory),
            max_age,
        }
    }
    
    /// Check if the value is stale and needs to be refreshed
    pub async fn is_stale(&self) -> Result<bool, LockError> {
        self.last_refreshed.with_read(|last| {
            match last {
                Some(instant) => instant.elapsed() > self.max_age,
                None => true, // If never refreshed, it's stale
            }
        }).await
    }
    
    /// Force a refresh of the value
    pub async fn refresh(&self) -> Result<(), LockError> {
        let new_value = (self.factory)();
        
        // Update the value and last_refreshed timestamp
        self.inner.set(new_value).await?;
        let _ = self.last_refreshed.set(Some(std::time::Instant::now())).await;
        
        Ok(())
    }
    
    /// Refresh the value if it's stale
    pub async fn ensure_fresh(&self) -> Result<bool, LockError> {
        // Fast path: check if we need to refresh
        let needs_refresh = match self.is_stale().await {
            Ok(stale) => stale,
            Err(e) => {
                warn!(name = %self.inner.name(), error = %e, "Failed to check if value is stale");
                true // Refresh anyway if we can't check
            }
        };
        
        if needs_refresh {
            debug!(name = %self.inner.name(), "Refreshing stale value");
            self.refresh().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Read the value, apply a function to produce a result, and return the result
    pub async fn with_read<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
    {
        self.ensure_fresh().await?;
        self.inner.with_read(f).await
    }
    
    /// Get the maximum age before the value should be refreshed
    pub fn max_age(&self) -> Duration {
        self.max_age
    }
    
    /// Set the maximum age before the value should be refreshed
    pub fn set_max_age(&mut self, max_age: Duration) {
        self.max_age = max_age;
    }
    
    /// Get a clone of the inner value, refreshing it if needed
    pub async fn get_cloned(&self) -> Result<T, LockError>
    where
        T: Clone,
    {
        self.ensure_fresh().await?;
        self.inner.get_cloned().await
    }
}

impl<T> fmt::Debug for RefreshableState<T>
where
    T: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshableState")
            .field("inner", &self.inner)
            .field("max_age", &format!("{:?}", self.max_age))
            .finish()
    }
}

// Unit tests for SharedState
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_shared_state_basic() {
        let state = SharedState::new(42);
        
        // Read
        let value = state.with_read(|v| *v).await.unwrap();
        assert_eq!(value, 42);
        
        // Write
        state.with_write(|v| *v = 99).await.unwrap();
        
        // Read again
        let value = state.with_read(|v| *v).await.unwrap();
        assert_eq!(value, 99);
    }
    
    #[tokio::test]
    async fn test_optional_shared_state() {
        let state = OptionalSharedState::<String>::new();
        
        // Check uninitialized
        assert!(!state.is_initialized().await.unwrap());
        
        // Try to read uninitialized
        let result = state.with_read(|_| ()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LockError::Uninitialized));
        
        // Initialize
        state.initialize(String::from("hello")).await.unwrap();
        
        // Check initialized
        assert!(state.is_initialized().await.unwrap());
        
        // Read
        let value = state.get_cloned().await.unwrap();
        assert_eq!(value, "hello");
    }
    
    #[tokio::test]
    async fn test_refreshable_state() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let refreshable = RefreshableState::new(
            0, 
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst) + 100
            },
            Duration::from_millis(50)
        );
        
        // Initial value
        let value = refreshable.with_read(|v| *v).await.unwrap();
        assert_eq!(value, 0);
        
        // Wait for it to become stale
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Read again (should refresh)
        let value = refreshable.with_read(|v| *v).await.unwrap();
        assert_eq!(value, 100);
        
        // Force refresh
        refreshable.refresh().await.unwrap();
        let value = refreshable.with_read(|v| *v).await.unwrap();
        assert_eq!(value, 101);
    }
}