use dashmap::{DashMap, DashSet};
use std::hash::Hash;
use tracing::{trace, warn};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    #[error("Key not found in map")]
    KeyNotFound,

    #[error("Operation failed: {message}")]
    OperationFailed { message: String, critical: bool },
}

impl MapError {
    pub fn operation_failed(message: impl Into<String>) -> Self {
        MapError::OperationFailed {
            message: message.into(),
            critical: false,
        }
    }

    pub fn critical_operation_failed(message: impl Into<String>) -> Self {
        MapError::OperationFailed {
            message: message.into(),
            critical: true,
        }
    }

    pub fn is_key_not_found(&self) -> bool {
        matches!(self, MapError::KeyNotFound)
    }

    pub fn is_critical(&self) -> bool {
        match self {
            MapError::OperationFailed { critical, .. } => *critical,
            _ => false,
        }
    }
}

pub type ConcurrentMap<K, V> = DashMap<K, V>;
pub type ConcurrentSet<T> = DashSet<T>;

pub trait MapExtensions<K, V> {
    fn get_cloned(&self, key: &K) -> Option<V>;
    fn get_cloned_or_error(&self, key: &K) -> Result<V, MapError>;
    fn modify<F, R>(&self, key: &K, f: F) -> Result<R, MapError>
    where
        F: FnOnce(&mut V) -> R;
}

impl<K, V> MapExtensions<K, V> for DashMap<K, V>
where
    K: Eq + Hash + Clone + std::fmt::Debug,
    V: Clone,
{
    fn get_cloned(&self, key: &K) -> Option<V> {
        self.get(key).map(|value| value.clone())
    }

    fn get_cloned_or_error(&self, key: &K) -> Result<V, MapError> {
        match self.get(key) {
            Some(value) => Ok(value.clone()),
            None => {
                warn!(?key, "Key not found in map");
                Err(MapError::KeyNotFound)
            }
        }
    }

    fn modify<F, R>(&self, key: &K, f: F) -> Result<R, MapError>
    where
        F: FnOnce(&mut V) -> R,
    {
        match self.get_mut(key) {
            Some(mut entry) => {
                trace!(?key, "Modifying value in map");
                Ok(f(&mut entry))
            }
            None => {
                warn!(?key, "Key not found for modification");
                Err(MapError::KeyNotFound)
            }
        }
    }
}

pub trait SetExtensions<T> {
    fn values(&self) -> Vec<T>;
}

impl<T> SetExtensions<T> for DashSet<T>
where
    T: Eq + Hash + Clone,
{
    fn values(&self) -> Vec<T> {
        self.iter().map(|value| value.clone()).collect()
    }
}
