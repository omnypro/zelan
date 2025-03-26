use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use thiserror::Error;
use tracing::{debug, error, trace, warn};

/// Errors that can occur during map operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// Key not found in the map
    #[error("Key not found in map")]
    KeyNotFound,
    
    /// Error during operation on a value
    #[error("Operation failed: {message}")]
    OperationFailed {
        /// Error message
        message: String,
        /// Whether this error is critical
        critical: bool,
    },
}

impl MapError {
    /// Create a new operation failed error
    pub fn operation_failed(message: impl Into<String>) -> Self {
        MapError::OperationFailed {
            message: message.into(),
            critical: false,
        }
    }
    
    /// Create a new critical operation failed error
    pub fn critical_operation_failed(message: impl Into<String>) -> Self {
        MapError::OperationFailed {
            message: message.into(),
            critical: true,
        }
    }
    
    /// Returns true if this is a key not found error
    pub fn is_key_not_found(&self) -> bool {
        matches!(self, MapError::KeyNotFound)
    }
    
    /// Returns true if this is a critical error
    pub fn is_critical(&self) -> bool {
        match self {
            MapError::OperationFailed { critical, .. } => *critical,
            _ => false,
        }
    }
}

/// A thread-safe, concurrent key-value store
/// This is a thin wrapper around DashMap
#[derive(Clone)]
pub struct ConcurrentMap<K, V> 
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// The underlying map storage
    inner: Arc<DashMap<K, V>>,
    /// Debug name for this map (used in logging)
    name: String,
}

impl<K, V> ConcurrentMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + fmt::Debug + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new, empty ConcurrentMap
    pub fn new() -> Self {
        Self::with_name("unnamed")
    }
    
    /// Create a new ConcurrentMap with the given name
    pub fn with_name(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!(name = %name, "Creating new ConcurrentMap");
        
        Self {
            inner: Arc::new(DashMap::new()),
            name,
        }
    }
    
    /// Create a new ConcurrentMap with the given initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_name_and_capacity("unnamed", capacity)
    }
    
    /// Create a new ConcurrentMap with the given name and initial capacity
    pub fn with_name_and_capacity(name: impl Into<String>, capacity: usize) -> Self {
        let name = name.into();
        debug!(name = %name, capacity, "Creating new ConcurrentMap with capacity");
        
        Self {
            inner: Arc::new(DashMap::with_capacity(capacity)),
            name,
        }
    }
    
    /// Get the number of key-value pairs in the map
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// Returns true if the map contains no elements
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Insert a key-value pair into the map, returning the previous value if any
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        trace!(name = %self.name, ?key, "Inserting value into map");
        self.inner.insert(key, value)
    }
    
    /// Remove a key-value pair from the map, returning the value if it was present
    pub fn remove(&self, key: &K) -> Option<V> {
        trace!(name = %self.name, ?key, "Removing value from map");
        self.inner.remove(key).map(|(_, v)| v)
    }
    
    /// Check if the map contains a key
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }
    
    /// Get a value from the map by cloning it (returns None if the key doesn't exist)
    pub fn get_cloned(&self, key: &K) -> Option<V> {
        trace!(name = %self.name, ?key, "Getting cloned value from map");
        self.inner.get(key).map(|value| value.clone())
    }
    
    /// Get a value from the map, or return an error if the key doesn't exist
    pub fn get_cloned_or_error(&self, key: &K) -> Result<V, MapError> {
        match self.get_cloned(key) {
            Some(value) => Ok(value),
            None => {
                warn!(name = %self.name, ?key, "Key not found in map");
                Err(MapError::KeyNotFound)
            }
        }
    }
    
    /// Get a value from the map, or insert a default if the key doesn't exist
    pub fn get_or_insert(&self, key: K, default: V) -> V {
        trace!(name = %self.name, ?key, "Getting or inserting value");
        let entry = self.inner.entry(key).or_insert(default);
        entry.clone()
    }
    
    /// Get a value from the map, or insert a value produced by a function if the key doesn't exist
    pub fn get_or_insert_with<F>(&self, key: K, default_fn: F) -> V
    where
        F: FnOnce() -> V,
    {
        trace!(name = %self.name, ?key, "Getting or inserting value with function");
        let entry = self.inner.entry(key).or_insert_with(default_fn);
        entry.clone()
    }
    
    /// Modify a value in the map, returning the result
    pub fn modify<F, R>(&self, key: &K, f: F) -> Result<R, MapError>
    where
        F: FnOnce(&mut V) -> R,
    {
        match self.inner.get_mut(key) {
            Some(mut entry) => {
                trace!(name = %self.name, ?key, "Modifying value in map");
                Ok(f(&mut entry))
            }
            None => {
                warn!(name = %self.name, ?key, "Key not found for modification");
                Err(MapError::KeyNotFound)
            }
        }
    }
    
    /// Apply a function to all key-value pairs in the map
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        trace!(name = %self.name, "Applying function to all values");
        self.inner.iter().for_each(|entry| f(entry.key(), entry.value()));
    }
    
    /// Apply a mutable function to all key-value pairs in the map
    pub fn for_each_mut<F>(&self, mut f: F)
    where
        F: FnMut(&K, &mut V),
    {
        trace!(name = %self.name, "Applying mutable function to all values");
        self.inner.iter_mut().for_each(|mut entry| {
            let key = entry.key().clone();
            f(&key, entry.value_mut())
        });
    }
    
    /// Remove all key-value pairs from the map
    pub fn clear(&self) {
        trace!(name = %self.name, "Clearing map");
        self.inner.clear();
    }
    
    /// Get all keys in the map
    pub fn keys(&self) -> Vec<K> {
        trace!(name = %self.name, "Getting all keys");
        self.inner.iter().map(|entry| entry.key().clone()).collect()
    }
    
    /// Get all values in the map
    pub fn values(&self) -> Vec<V> {
        trace!(name = %self.name, "Getting all values");
        self.inner.iter().map(|entry| entry.value().clone()).collect()
    }
    
    /// Get all key-value pairs in the map
    pub fn entries(&self) -> Vec<(K, V)> {
        trace!(name = %self.name, "Getting all key-value pairs");
        self.inner.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
    }
    
    /// Get access to the inner DashMap for advanced operations
    pub fn inner(&self) -> &Arc<DashMap<K, V>> {
        &self.inner
    }
}

/// A thread-safe, concurrent set implementation
/// This is a thin wrapper around DashSet
#[derive(Clone)]
pub struct ConcurrentSet<T>
where
    T: Eq + Hash + Clone + Send + Sync + 'static,
{
    /// The underlying set storage
    inner: Arc<DashSet<T>>,
    /// Debug name for this set (used in logging)
    name: String,
}

impl<T> ConcurrentSet<T>
where
    T: Eq + Hash + Clone + Send + Sync + fmt::Debug + 'static,
{
    /// Create a new, empty ConcurrentSet
    pub fn new() -> Self {
        Self::with_name("unnamed")
    }
    
    /// Create a new ConcurrentSet with the given name
    pub fn with_name(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!(name = %name, "Creating new ConcurrentSet");
        
        Self {
            inner: Arc::new(DashSet::new()),
            name,
        }
    }
    
    /// Create a new ConcurrentSet with the given initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_name_and_capacity("unnamed", capacity)
    }
    
    /// Create a new ConcurrentSet with the given name and initial capacity
    pub fn with_name_and_capacity(name: impl Into<String>, capacity: usize) -> Self {
        let name = name.into();
        debug!(name = %name, capacity, "Creating new ConcurrentSet with capacity");
        
        Self {
            inner: Arc::new(DashSet::with_capacity(capacity)),
            name,
        }
    }
    
    /// Get the number of values in the set
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// Returns true if the set contains no elements
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Insert a value into the set, returning true if the value was not already present
    pub fn insert(&self, value: T) -> bool {
        trace!(name = %self.name, ?value, "Inserting value into set");
        self.inner.insert(value)
    }
    
    /// Remove a value from the set, returning true if the value was present
    pub fn remove(&self, value: &T) -> bool {
        trace!(name = %self.name, ?value, "Removing value from set");
        self.inner.remove(value).is_some()
    }
    
    /// Check if the set contains a value
    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }
    
    /// Clear all values from the set
    pub fn clear(&self) {
        trace!(name = %self.name, "Clearing set");
        self.inner.clear();
    }
    
    /// Get all values in the set
    pub fn values(&self) -> Vec<T> {
        trace!(name = %self.name, "Getting all values");
        self.inner.iter().map(|value| value.clone()).collect()
    }
    
    /// Apply a function to all values in the set
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        trace!(name = %self.name, "Applying function to all values");
        self.inner.iter().for_each(|value| f(&*value));
    }
    
    /// Get access to the inner DashSet for advanced operations
    pub fn inner(&self) -> &Arc<DashSet<T>> {
        &self.inner
    }
}

// Unit tests for ConcurrentMap and ConcurrentSet
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_concurrent_map_basic() {
        let map = ConcurrentMap::<String, i32>::new();
        
        // Insert
        assert_eq!(map.insert("one".to_string(), 1), None);
        assert_eq!(map.insert("two".to_string(), 2), None);
        
        // Get
        assert_eq!(map.get_cloned(&"one".to_string()), Some(1));
        assert_eq!(map.get_cloned(&"two".to_string()), Some(2));
        assert_eq!(map.get_cloned(&"three".to_string()), None);
        
        // Contains
        assert!(map.contains_key(&"one".to_string()));
        assert!(!map.contains_key(&"three".to_string()));
        
        // Remove
        assert_eq!(map.remove(&"one".to_string()), Some(1));
        assert!(!map.contains_key(&"one".to_string()));
        
        // Len
        assert_eq!(map.len(), 1);
    }
    
    #[tokio::test]
    async fn test_concurrent_map_modify() {
        let map = ConcurrentMap::<String, i32>::new();
        map.insert("key".to_string(), 10);
        
        // Modify
        let result = map.modify(&"key".to_string(), |value| {
            *value += 5;
            *value
        });
        assert_eq!(result, Ok(15));
        
        // Try to modify non-existent key
        let err = map.modify(&"missing".to_string(), |_| ()).unwrap_err();
        assert!(err.is_key_not_found());
        
        // Get after modification
        assert_eq!(map.get_cloned(&"key".to_string()), Some(15));
    }
    
    #[tokio::test]
    async fn test_concurrent_map_direct_access() {
        let map = ConcurrentMap::<String, i32>::new();
        
        // Access directly through inner
        map.inner().insert("direct".to_string(), 42);
        
        // Should be accessible through the wrapper methods
        assert_eq!(map.get_cloned(&"direct".to_string()), Some(42));
    }
    
    #[tokio::test]
    async fn test_concurrent_set_basic() {
        let set = ConcurrentSet::<String>::new();
        
        // Insert
        assert!(set.insert("one".to_string()));
        assert!(set.insert("two".to_string()));
        assert!(!set.insert("one".to_string())); // Already exists
        
        // Contains
        assert!(set.contains(&"one".to_string()));
        assert!(!set.contains(&"three".to_string()));
        
        // Remove
        assert!(set.remove(&"one".to_string()));
        assert!(!set.contains(&"one".to_string()));
        
        // Len
        assert_eq!(set.len(), 1);
        
        // Values
        let values = set.values();
        assert_eq!(values.len(), 1);
        assert!(values.contains(&"two".to_string()));
    }
}