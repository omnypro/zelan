use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::RwLock;
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
    },
}

/// A thread-safe wrapper around shared state
#[derive(Clone)]
pub struct SharedState<T>
where
    T: Send + Sync + 'static
{
    /// The underlying shared state
    inner: Arc<RwLock<T>>,
    /// Debug name for this shared state (used in logging)
    name: String,
}

impl<T> SharedState<T>
where
    T: Send + Sync + 'static
{
    /// Create a new SharedState with the given initial value
    pub fn new(value: T) -> Self {
        Self::with_name(value, "unnamed")
    }
    
    /// Create a new SharedState with the given name
    pub fn with_name(value: T, name: impl Into<String>) -> Self {
        let name = name.into();
        debug!(name = %name, "Creating new SharedState");
        
        Self {
            inner: Arc::new(RwLock::new(value)),
            name,
        }
    }
    
    /// Get the name of this SharedState
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Read the value, apply a function to produce a result, and return the result
    pub async fn read<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
        R: Send + 'static,
    {
        trace!(name = %self.name, "Acquiring read lock for function");
        
        let guard = self.inner.read().await;
        trace!(name = %self.name, "Read lock acquired for function");
        let result = f(&guard);
        Ok(result)
    }
    
    /// Write to the value, apply a function to it, and return a result
    pub async fn write<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&mut T) -> R,
        R: Send + 'static,
    {
        trace!(name = %self.name, "Acquiring write lock for function");
        
        let mut guard = self.inner.write().await;
        trace!(name = %self.name, "Write lock acquired for function");
        let result = f(&mut guard);
        Ok(result)
    }
    
    /// Get a clone of the inner value
    pub async fn get_cloned(&self) -> Result<T, LockError>
    where
        T: Clone,
    {
        self.read(|value| value.clone()).await
    }
    
    /// Set the inner value
    pub async fn set(&self, value: T) -> Result<(), LockError> {
        self.write(|inner| *inner = value).await
    }
    
    /// Update the inner value using a function
    pub async fn update<F>(&self, f: F) -> Result<(), LockError>
    where
        F: FnOnce(&mut T),
    {
        self.write(|inner| f(inner)).await
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
        self.inner.read(|opt| opt.is_some()).await
    }
    
    /// Read the value, apply a function to produce a result, and return the result
    /// Returns an error if the state is not initialized
    pub async fn with_read<F, R>(&self, f: F) -> Result<R, LockError>
    where
        F: FnOnce(&T) -> R,
        R: Send + 'static,
    {
        self.inner.read(|opt| {
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
        R: Send + 'static,
    {
        self.inner.write(|opt| {
            match opt {
                Some(value) => Ok(f(value)),
                None => Err(LockError::Uninitialized),
            }
        }).await?
    }
    
    /// Initialize the state with a value
    pub async fn initialize(&self, value: T) -> Result<(), LockError> {
        self.inner.write(|opt| {
            *opt = Some(value);
        }).await
    }
    
    /// Initialize the state with a value if it's not already initialized
    pub async fn initialize_if_empty(&self, value: T) -> Result<bool, LockError> {
        self.inner.write(|opt| {
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
        self.inner.write(|opt| {
            *opt = None;
        }).await
    }
    
    /// Take the value, leaving the state uninitialized
    /// Returns an error if the state is not initialized
    pub async fn take(&self) -> Result<T, LockError> {
        self.inner.write(|opt| {
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
        self.last_refreshed.read(|last| {
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
        self.last_refreshed.set(Some(std::time::Instant::now())).await?;
        
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
        R: Send + 'static,
    {
        self.ensure_fresh().await?;
        self.inner.read(f).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[tokio::test]
    async fn test_shared_state_basic() {
        let state = SharedState::new(42);
        
        // Read
        let value = state.read(|v| *v).await.unwrap();
        assert_eq!(value, 42);
        
        // Write
        state.write(|v| *v = 99).await.unwrap();
        
        // Read again
        let value = state.read(|v| *v).await.unwrap();
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