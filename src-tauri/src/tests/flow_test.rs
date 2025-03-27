use std::time::Duration;
use std::thread;

use crate::flow::{TraceContext, TraceRegistry};

#[test]
fn test_trace_context_creation() {
    let trace = TraceContext::new("test_source".to_string(), "test_event".to_string());
    
    assert_eq!(trace.source, "test_source");
    assert_eq!(trace.event_type, "test_event");
    assert!(trace.spans.is_empty());
    assert!(trace.path.is_empty());
}

#[test]
fn test_adding_spans() {
    let mut trace = TraceContext::new("test_source".to_string(), "test_event".to_string());
    
    trace.add_span("publish", "EventBus");
    assert_eq!(trace.spans.len(), 1);
    assert_eq!(trace.path.len(), 1);
    assert_eq!(trace.path[0], "EventBus");
    
    // Add a second span
    trace.add_span("callback", "TwitchAdapter");
    assert_eq!(trace.spans.len(), 2);
    assert_eq!(trace.path.len(), 2);
    assert_eq!(trace.path[1], "TwitchAdapter");
}

#[test]
fn test_completing_spans() {
    let mut trace = TraceContext::new("test_source".to_string(), "test_event".to_string());
    
    trace.add_span("publish", "EventBus");
    thread::sleep(Duration::from_millis(10));
    trace.complete_span();
    
    assert!(trace.spans[0].duration_ms.is_some());
    assert!(trace.spans[0].duration_ms.unwrap() >= 10);
}

#[tokio::test]
async fn test_trace_registry() {
    let registry = TraceRegistry::new(5);
    
    // Add traces
    for i in 0..3 {
        let mut trace = TraceContext::new(format!("source_{}", i), format!("event_{}", i));
        trace.add_span("test", "test_component");
        thread::sleep(Duration::from_millis(5));
        trace.complete_span();
        trace.complete();
        registry.record_trace(trace).await;
    }
    
    // Check that we can retrieve them
    let traces = registry.get_recent_traces(10).await;
    assert_eq!(traces.len(), 3);
    
    // Test capacity limit
    for i in 0..5 {
        let mut trace = TraceContext::new(format!("source_extra_{}", i), format!("event_extra_{}", i));
        trace.add_span("test", "test_component");
        thread::sleep(Duration::from_millis(5));
        trace.complete_span();
        trace.complete();
        registry.record_trace(trace).await;
    }
    
    // Should only keep the most recent 5
    let traces = registry.get_recent_traces(10).await;
    assert_eq!(traces.len(), 5);
    assert_eq!(traces[0].source, "source_extra_0");
}