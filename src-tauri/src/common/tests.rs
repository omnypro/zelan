//! Tests for common module components

#[cfg(test)]
mod shared_state_tests {
    use super::super::shared_state::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_shared_state_basic() {
        let state = SharedState::new(42);
        
        // Read
        let value = state.read(|v| *v).await;
        assert_eq!(value, 42);
        
        // Write
        state.write(|v| *v = 99).await;
        
        // Read again
        let value = state.read(|v| *v).await;
        assert_eq!(value, 99);
    }
    
    #[tokio::test]
    async fn test_optional_shared_state() {
        let state = OptionalSharedState::<String>::new();
        
        // Check uninitialized
        assert!(!state.is_initialized().await);
        
        // Initialize
        state.initialize(String::from("hello")).await;
        
        // Check initialized
        assert!(state.is_initialized().await);
        
        // Read using with_read
        let value = state.with_read(|v| v.clone()).await.unwrap();
        assert_eq!(value, "hello");
    }
    
    #[tokio::test]
    async fn test_refreshable_state() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let refreshable = RefreshableState::new(
            0, 
            move || {
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 100
            },
            Duration::from_millis(50)
        );
        
        // Initial value
        let value = refreshable.with_read(|v| *v).await;
        assert_eq!(value, 0);
        
        // Wait for it to become stale
        sleep(Duration::from_millis(100)).await;
        
        // Read again (should refresh)
        let value = refreshable.with_read(|v| *v).await;
        assert_eq!(value, 100);
        
        // Force refresh
        refreshable.refresh().await;
        let value = refreshable.with_read(|v| *v).await;
        assert_eq!(value, 101);
    }
}

#[cfg(test)]
mod concurrent_map_tests {
    use super::super::concurrent_map::*;
    
    #[tokio::test]
    async fn test_concurrent_map() {
        let map = ConcurrentMap::<String, i32>::new();
        
        // Insert
        map.insert("one".to_string(), 1);
        map.insert("two".to_string(), 2);
        
        // Get
        assert_eq!(map.get_cloned(&"one".to_string()), Some(1));
        assert_eq!(map.get_cloned(&"three".to_string()), None::<i32>);
        
        // Modify
        let result = map.modify(&"one".to_string(), |v: &mut i32| {
            *v += 10;
            *v
        }).unwrap();
        assert_eq!(result, 11);
        
        // Check after modification
        assert_eq!(map.get_cloned(&"one".to_string()), Some(11));
        
        // Try to modify missing key
        let err = map.modify(&"missing".to_string(), |_: &mut i32| ()).unwrap_err();
        assert!(err.is_key_not_found());
        
        // Remove
        let removed = map.remove(&"one".to_string());
        assert_eq!(removed, Some(11));
        assert!(!map.contains_key(&"one".to_string()));
    }
    
    #[tokio::test]
    async fn test_concurrent_set() {
        let set: ConcurrentSet<String> = ConcurrentSet::new();
        
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
        assert!(set.remove(&"one".to_string()));
        assert!(!set.contains(&"one".to_string()));
        
        // Values
        let values: Vec<String> = set.values();
        assert_eq!(values.len(), 1);
        assert!(values.contains(&"two".to_string()));
    }
    
    // BatchOperation is planned for future implementation
    // Commenting out this test until implemented
    /*
    #[tokio::test]
    async fn test_batch_operation() {
        let map = ConcurrentMap::<String, i32>::new();
        
        // Create a batch operation
        let result = BatchOperation::new(map.clone())
            .insert("a".to_string(), 1)
            .insert("b".to_string(), 2)
            .modify("a".to_string(), |v: &mut i32| *v += 10)
            .execute();
        
        assert!(result.is_ok());
        
        // Check results
        assert_eq!(map.get_cloned(&"a".to_string()), Some(11));
        assert_eq!(map.get_cloned(&"b".to_string()), Some(2));
    }
    */
}