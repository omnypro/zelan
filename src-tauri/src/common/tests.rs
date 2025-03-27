//! Tests for common module components

#[cfg(test)]
mod rwlock_tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_rwlock_basic() {
        let state = Arc::new(RwLock::new(42));

        // Read
        {
            let value = *state.read().await;
            assert_eq!(value, 42);
        }

        // Write
        {
            let mut write_guard = state.write().await;
            *write_guard = 99;
        }

        // Read again
        {
            let value = *state.read().await;
            assert_eq!(value, 99);
        }
    }

    #[tokio::test]
    async fn test_optional_value() {
        let state = Arc::new(RwLock::new(Option::<String>::None));

        // Check uninitialized
        {
            let value = state.read().await;
            assert!(value.is_none());
        }

        // Initialize
        {
            let mut value = state.write().await;
            *value = Some(String::from("hello"));
        }

        // Check initialized
        {
            let value = state.read().await;
            assert!(value.is_some());
            assert_eq!(value.as_ref().unwrap(), "hello");
        }
    }
}

#[cfg(test)]
mod concurrent_map_tests {
    use dashmap::{DashMap, DashSet};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_concurrent_map() {
        let map = Arc::new(DashMap::<String, i32>::new());

        // Insert
        map.insert("one".to_string(), 1);
        map.insert("two".to_string(), 2);

        // Get
        assert_eq!(map.get(&"one".to_string()).map(|r| *r), Some(1));
        assert_eq!(map.get(&"three".to_string()).map(|r| *r), None::<i32>);

        // Modify
        if let Some(mut entry) = map.get_mut(&"one".to_string()) {
            *entry += 10;
            assert_eq!(*entry, 11);
        }

        // Check after modification
        assert_eq!(map.get(&"one".to_string()).map(|r| *r), Some(11));

        // Try to modify missing key
        assert!(map.get_mut(&"missing".to_string()).is_none());

        // Remove
        let removed = map.remove(&"one".to_string()).map(|(_, v)| v);
        assert_eq!(removed, Some(11));
        assert!(!map.contains_key(&"one".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_set() {
        let set = Arc::new(DashSet::<String>::new());

        // Insert
        assert!(set.insert("one".to_string()));
        assert!(set.insert("two".to_string()));
        assert!(!set.insert("one".to_string())); // Already exists

        // Contains
        assert!(set.contains(&"one".to_string()));
        assert!(!set.contains(&"three".to_string()));

        // Len
        assert_eq!(set.len(), 2);

        // Remove
        assert!(set.remove(&"one".to_string()).is_some());
        assert!(!set.contains(&"one".to_string()));

        // Values
        let values: Vec<String> = set.iter().map(|item| item.clone()).collect();
        assert_eq!(values.len(), 1);
        assert!(values.contains(&"two".to_string()));
    }
}
