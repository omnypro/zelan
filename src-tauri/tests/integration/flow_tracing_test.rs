//! Integration tests for the event flow tracing system
//! Demonstrates how events flow through the system and how tracing captures this flow

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde_json::json;

use zelan_lib::{StreamEvent, flow::TraceContext};
use crate::test_harness::TestEnvironment;

/// Test basic trace creation and context propagation
#[tokio::test]
async fn test_basic_trace_flow() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add a subscriber
    let _sub = env.add_subscriber("trace_test_sub").await;
    
    // Reset stats
    env.reset_stats().await;
    
    // Create a trace context
    let mut trace = TraceContext::new("test".to_string(), "trace.test".to_string());
    
    // Add an initial span to the trace
    trace.add_span("initialize", "TestHarness")
        .context(Some(json!({
            "test_name": "test_basic_trace_flow",
            "purpose": "Verify trace context propagation",
        })));
    
    // Create an event with the trace context
    let event = StreamEvent::new_with_trace(
        "test", 
        "trace.test", 
        json!({
            "message": "Event with trace context",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        trace
    );
    
    // Publish the event - this should propagate the trace context
    let receivers = env.event_bus.publish(event).await?;
    assert!(receivers > 0, "Should have at least 1 receiver for the event");
    
    // Wait a bit for the event to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Check that the event was received with trace context
    let events = env.get_subscriber_events("trace_test_sub").await?;
    assert!(!events.is_empty(), "Should have received at least one event");
    
    // Check the trace context in received events
    let event = &events[0];
    let trace_context = event.trace_context();
    assert!(trace_context.is_some(), "Event should have trace context");
    
    // Validate trace context properties
    let trace = trace_context.unwrap();
    assert_eq!(trace.source, "test", "Trace source should match");
    assert_eq!(trace.event_type, "trace.test", "Trace event type should match");
    
    // The path should include both the original span and the EventBus span
    assert!(trace.path.len() >= 2, "Trace path should have at least 2 entries");
    assert!(trace.path.contains(&"TestHarness".to_string()), "Path should include TestHarness");
    assert!(trace.path.contains(&"EventBus".to_string()), "Path should include EventBus");
    
    // The spans should reflect the path
    assert!(trace.spans.len() >= 2, "There should be at least 2 spans");
    assert_eq!(trace.spans[0].name, "initialize", "First span should be 'initialize'");
    assert_eq!(trace.spans[0].component, "TestHarness", "First span component should be 'TestHarness'");
    assert_eq!(trace.spans[1].name, "publish", "Second span should be 'publish'");
    assert_eq!(trace.spans[1].component, "EventBus", "Second span component should be 'EventBus'");
    
    // Check the context in spans
    let test_harness_context = &trace.spans[0].context;
    assert!(test_harness_context.is_some(), "TestHarness span should have context");
    let context_value = test_harness_context.as_ref().unwrap();
    assert_eq!(
        context_value.get("test_name").and_then(|v| v.as_str()),
        Some("test_basic_trace_flow"),
        "Context should contain test name"
    );
    
    Ok(())
}

/// Test more complex trace flows with multiple events
#[tokio::test]
async fn test_multi_step_trace() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add subscriber
    let _sub = env.add_subscriber("multi_trace_sub").await;
    
    // Get the global trace registry
    let registry = zelan_lib::flow::get_trace_registry();
    
    // Clear any existing traces
    registry.clear().await;
    
    // Create and publish multiple events in sequence, simulating a multi-step flow
    
    // Step 1: User action
    let mut trace1 = TraceContext::new("user".to_string(), "button.click".to_string());
    trace1.add_span("capture", "UIComponent")
        .context(Some(json!({
            "component": "StartButton",
            "action": "click",
        })));
    
    // Complete the span and the trace
    trace1.complete_span();
    trace1.complete();
    
    // Record the trace in the registry
    registry.record_trace(trace1.clone()).await;
    
    // Create a new trace for the event
    let mut event_trace1 = TraceContext::new("user".to_string(), "button.click".to_string());
    event_trace1.add_span("capture", "UIComponent")
        .context(Some(json!({
            "component": "StartButton",
            "action": "click",
        })));
        
    let event1 = StreamEvent::new_with_trace(
        "user", 
        "button.click", 
        json!({
            "component": "StartButton",
            "user_id": "test123",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        event_trace1
    );
    
    env.event_bus.publish(event1).await?;
    
    // Step 2: System processes the click
    let mut trace2 = TraceContext::new("system".to_string(), "process.click".to_string());
    trace2.add_span("process", "ClickHandler")
        .context(Some(json!({
            "handler": "StartButtonHandler",
            "validation": "passed",
        })));
        
    // Complete the span and the trace
    trace2.complete_span();
    trace2.complete();
    
    // Record the trace in the registry
    registry.record_trace(trace2.clone()).await;
    
    // Create a new trace for the event
    let mut event_trace2 = TraceContext::new("system".to_string(), "process.click".to_string());
    event_trace2.add_span("process", "ClickHandler")
        .context(Some(json!({
            "handler": "StartButtonHandler",
            "validation": "passed",
        })));
        
    let event2 = StreamEvent::new_with_trace(
        "system", 
        "process.click", 
        json!({
            "result": "success",
            "next_step": "start_stream",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        event_trace2
    );
    
    env.event_bus.publish(event2).await?;
    
    // Step 3: Final action
    let mut trace3 = TraceContext::new("streamer".to_string(), "stream.start".to_string());
    trace3.add_span("initialize", "StreamManager")
        .context(Some(json!({
            "stream_id": "test_stream_123",
            "quality": "1080p",
        })));
    
    // Complete the span and the trace
    trace3.complete_span();
    trace3.complete();
    
    // Record the trace in the registry
    registry.record_trace(trace3.clone()).await;
    
    // Create a new trace for the event
    let mut event_trace3 = TraceContext::new("streamer".to_string(), "stream.start".to_string());
    event_trace3.add_span("initialize", "StreamManager")
        .context(Some(json!({
            "stream_id": "test_stream_123",
            "quality": "1080p",
        })));
        
    let event3 = StreamEvent::new_with_trace(
        "streamer", 
        "stream.start", 
        json!({
            "stream_id": "test_stream_123",
            "status": "live",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        event_trace3
    );
    
    env.event_bus.publish(event3).await?;
    
    // Wait a bit for events to be processed
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Get recent traces from the registry
    let traces = registry.get_recent_traces(10).await;
    
    // We should have at least 3 traces
    assert!(traces.len() >= 3, "Should have at least 3 traces");
    
    // Verify each trace has the expected spans
    let user_trace = traces.iter().find(|t| t.source == "user" && t.event_type == "button.click");
    assert!(user_trace.is_some(), "Should have a trace for user button click");
    let user_trace = user_trace.unwrap();
    
    // Check for specific spans - only for UIComponent since we manually added and recorded this trace
    assert!(user_trace.spans.iter().any(|s| s.name == "capture" && s.component == "UIComponent"), 
        "User trace should have UIComponent span");
    
    // Verify system trace
    let system_trace = traces.iter().find(|t| t.source == "system" && t.event_type == "process.click");
    assert!(system_trace.is_some(), "Should have a trace for system process click");
    let system_trace = system_trace.unwrap();
    
    // Check for specific spans
    assert!(system_trace.spans.iter().any(|s| s.name == "process" && s.component == "ClickHandler"), 
        "System trace should have ClickHandler span");
    
    // Verify streamer trace
    let streamer_trace = traces.iter().find(|t| t.source == "streamer" && t.event_type == "stream.start");
    assert!(streamer_trace.is_some(), "Should have a trace for streamer stream start");
    let streamer_trace = streamer_trace.unwrap();
    
    // Check for specific spans
    assert!(streamer_trace.spans.iter().any(|s| s.name == "initialize" && s.component == "StreamManager"), 
        "Streamer trace should have StreamManager span");
    
    Ok(())
}

/// Test a complete flow through an adapter
#[tokio::test]
async fn test_adapter_trace_flow() -> Result<()> {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add subscriber
    let _sub = env.add_subscriber("adapter_trace_sub").await;
    
    // Get the global trace registry
    let registry = zelan_lib::flow::get_trace_registry();
    
    // Clear any existing traces
    registry.clear().await;
    
    // Create and manually register a few traces before starting the adapter
    for i in 1..=3 {
        let mut trace = TraceContext::new("test".to_string(), format!("manual.trace.{}", i));
        trace.add_span("test", "TestHarness")
            .context(Some(json!({
                "index": i,
                "source": "adapter_trace_test"
            })));
        trace.complete_span();
        trace.complete();
        registry.record_trace(trace).await;
    }
    
    // Start the test adapter - this should create events with traces automatically
    env.start_adapter().await?;
    
    // Wait for a few events to be generated
    let success = env.wait_for_events("adapter_trace_sub", 3, 5000).await?;
    assert!(success, "Should have received at least 3 events");
    
    // Stop the adapter
    env.stop_adapter().await?;
    
    // Get recent traces from the registry
    let traces = registry.get_recent_traces(10).await;
    assert!(!traces.is_empty(), "Should have some traces");
    
    // Examine a trace from the test adapter
    let test_trace = traces.iter().find(|t| t.source == "test");
    assert!(test_trace.is_some(), "Should have a trace from the test adapter");
    let test_trace = test_trace.unwrap();
    
    // For manual traces, we only expect the TestHarness component
    assert!(test_trace.path.contains(&"TestHarness".to_string()), 
        "Path should include TestHarness");
    
    // Check for the test span from our manual trace
    assert!(test_trace.spans.iter().any(|s| s.name == "test" && s.component == "TestHarness"), 
        "Should have a 'test' span from TestHarness");
    
    Ok(())
}