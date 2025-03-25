//! Tests for the callback system
//! 
//! These tests verify that the callback system properly
//! maintains callback integrity across clone boundaries and async boundaries.

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use anyhow::Result;
    
    use crate::callback_system::{CallbackRegistry, CallbackManager};
    
    // Test data type
    #[derive(Clone, Debug)]
    struct TestEvent {
        id: usize,
        message: String,
    }
    
    #[tokio::test]
    async fn test_callback_registration() -> Result<()> {
        // Create a registry
        let registry = CallbackRegistry::<TestEvent>::new();
        
        // Track callback invocations
        let counter = Arc::new(AtomicUsize::new(0));
        
        // Register a callback
        let counter_clone = Arc::clone(&counter);
        let id = registry.register(move |event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("Event received: {:?}", event);
            Ok(())
        }).await;
        
        // Verify we have a callback ID
        assert!(!id.to_string().is_empty(), "Should have a valid callback ID");
        
        // Verify we have one callback registered
        assert_eq!(registry.count().await, 1, "Should have one callback registered");
        
        // Trigger the callback
        let event = TestEvent {
            id: 1,
            message: "Test event".to_string(),
        };
        
        let count = registry.trigger(event).await?;
        assert_eq!(count, 1, "Should have triggered one callback");
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Callback should have been invoked once");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_callback_clone_integrity() -> Result<()> {
        // Create a registry
        let registry = CallbackRegistry::<TestEvent>::new();
        
        // Track callback invocations
        let counter = Arc::new(AtomicUsize::new(0));
        
        // Register a callback
        let counter_clone = Arc::clone(&counter);
        let _id = registry.register(move |event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("Event received by original: {:?}", event);
            Ok(())
        }).await;
        
        // Create a clone of the registry
        let cloned_registry = registry.clone();
        
        // Verify the clone has the same callback count
        assert_eq!(cloned_registry.count().await, 1, "Clone should have same callback count");
        
        // Trigger the callback through the clone
        let event = TestEvent {
            id: 2,
            message: "Event via clone".to_string(),
        };
        
        let count = cloned_registry.trigger(event).await?;
        assert_eq!(count, 1, "Should have triggered one callback through clone");
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Callback should have been invoked once");
        
        // Register another callback on the clone
        let counter_clone = Arc::clone(&counter);
        let _id2 = cloned_registry.register(move |event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            println!("Event received by new callback: {:?}", event);
            Ok(())
        }).await;
        
        // Verify both registries now show 2 callbacks
        assert_eq!(registry.count().await, 2, "Original should see new callback");
        assert_eq!(cloned_registry.count().await, 2, "Clone should see both callbacks");
        
        // Trigger callbacks through original registry
        let event = TestEvent {
            id: 3,
            message: "Event via original".to_string(),
        };
        
        let count = registry.trigger(event).await?;
        assert_eq!(count, 2, "Should have triggered two callbacks");
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Callbacks should have been invoked 3 times total");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_callback_manager() -> Result<()> {
        // Create a callback manager
        let manager = CallbackManager::new();
        
        // Get two different registries
        let int_registry = manager.registry::<i32>("integers").await;
        let string_registry = manager.registry::<String>("strings").await;
        
        // Track callback invocations
        let int_counter = Arc::new(AtomicUsize::new(0));
        let string_counter = Arc::new(AtomicUsize::new(0));
        
        // Register callbacks
        let int_counter_clone = Arc::clone(&int_counter);
        int_registry.register(move |n| {
            int_counter_clone.fetch_add(n as usize, Ordering::SeqCst);
            Ok(())
        }).await;
        
        let string_counter_clone = Arc::clone(&string_counter);
        string_registry.register(move |s| {
            string_counter_clone.fetch_add(s.len(), Ordering::SeqCst);
            Ok(())
        }).await;
        
        // Trigger callbacks
        int_registry.trigger(42).await?;
        string_registry.trigger("hello".to_string()).await?;
        
        // Verify counters
        assert_eq!(int_counter.load(Ordering::SeqCst), 42, "Int callback should add 42");
        assert_eq!(string_counter.load(Ordering::SeqCst), 5, "String callback should add 5");
        
        // Check registry stats
        let registry_names = manager.registry_names().await;
        assert_eq!(registry_names.len(), 2, "Should have two registries");
        assert!(registry_names.contains(&"integers".to_string()), "Should have integers registry");
        assert!(registry_names.contains(&"strings".to_string()), "Should have strings registry");
        
        // Check counts
        assert_eq!(manager.registry_count("integers").await?, 1, "Integers registry should have 1 callback");
        assert_eq!(manager.registry_count("strings").await?, 1, "Strings registry should have 1 callback");
        
        Ok(())
    }
}