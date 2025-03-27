//! Event flow tracking system for observing reactive event flows
//! 
//! This module provides tools for tracking events as they flow through
//! the reactive system, including callbacks, event bus, and adapters.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Trace context that follows an event through its lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Unique identifier for this trace
    pub trace_id: Uuid,
    
    /// Original source component that created the event
    pub source: String,
    
    /// Event type being traced
    pub event_type: String,
    
    /// Path showing where the event has been processed
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    
    /// When the trace was created
    pub start_time: DateTime<Utc>,
    
    /// Individual trace spans showing each processing step
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<TraceSpan>,
    
    /// Whether this trace has completed its expected flow
    #[serde(skip)]
    pub completed: bool,
    
    /// Total processing time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<u64>,
}

/// A single span in the trace representing one step in processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Name of this span (e.g., "publish", "callback", "subscribe")
    pub name: String,
    
    /// Component that processed this span
    pub component: String,
    
    /// When this span started
    pub start_time: DateTime<Utc>,
    
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    
    /// Additional context about this span
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    
    /// For internal timing calculations
    #[serde(skip)]
    start_instant: Option<Instant>,
}

impl TraceSpan {
    /// Add context to this span
    pub fn context(&mut self, ctx: Option<serde_json::Value>) -> &mut Self {
        self.context = ctx;
        self
    }
}

impl TraceContext {
    /// Create a new trace context
    pub fn new(source: String, event_type: String) -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            source,
            event_type,
            path: Vec::new(),
            start_time: Utc::now(),
            spans: Vec::new(),
            completed: false,
            total_ms: None,
            
        }
    }
    
    /// Add a new span to the trace
    pub fn add_span(&mut self, name: &str, component: &str) -> &mut TraceSpan {
        let span = TraceSpan {
            name: name.to_string(),
            component: component.to_string(),
            start_time: Utc::now(),
            duration_ms: None,
            context: None,
            start_instant: Some(Instant::now()),
        };
        
        // Add component to path for easy visualization
        self.path.push(component.to_string());
        
        // Add the span
        self.spans.push(span);
        
        // Return a mutable reference to the span for further configuration
        self.spans.last_mut().unwrap()
    }
    
    /// Add context to the most recent span
    pub fn add_context(&mut self, context: serde_json::Value) -> &mut Self {
        if let Some(span) = self.spans.last_mut() {
            span.context = Some(context);
        }
        self
    }
    
    /// Get the most recently added span (for fluent API)
    pub fn last_span_mut(&mut self) -> Option<&mut TraceSpan> {
        self.spans.last_mut()
    }
    
    /// Complete the most recent span and record its duration
    pub fn complete_span(&mut self) -> &mut Self {
        if let Some(span) = self.spans.last_mut() {
            if let Some(start) = span.start_instant {
                let duration = start.elapsed();
                span.duration_ms = Some(duration.as_millis() as u64);
                
                // Log completion if it took more than 10ms
                if duration > Duration::from_millis(10) {
                    debug!(
                        trace_id = %self.trace_id,
                        span = %span.name,
                        component = %span.component,
                        duration_ms = %span.duration_ms.unwrap(),
                        "Completed span"
                    );
                }
            }
        }
        self
    }
    
    /// Mark the entire trace as completed
    pub fn complete(&mut self) {
        // Complete any incomplete spans
        if let Some(span) = self.spans.last_mut() {
            if span.duration_ms.is_none() && span.start_instant.is_some() {
                let duration = span.start_instant.unwrap().elapsed();
                span.duration_ms = Some(duration.as_millis() as u64);
            }
        }
        
        // Calculate total duration
        let total_duration = Instant::now().duration_since(
            self.spans.first()
                .and_then(|s| s.start_instant)
                .unwrap_or_else(Instant::now)
        );
        
        self.total_ms = Some(total_duration.as_millis() as u64);
        self.completed = true;
        
        // Log completion of the entire trace
        info!(
            trace_id = %self.trace_id,
            source = %self.source,
            event_type = %self.event_type,
            path = ?self.path,
            total_ms = %self.total_ms.unwrap(),
            spans = %self.spans.len(),
            "Completed event trace"
        );
    }
}

/// Event trace history for collecting and analyzing traces
#[derive(Debug)]
pub struct TraceRegistry {
    /// Recent traces
    traces: RwLock<Vec<TraceContext>>,
    
    /// Maximum number of traces to keep
    capacity: usize,
}

impl TraceRegistry {
    /// Create a new trace registry
    pub fn new(capacity: usize) -> Self {
        Self {
            traces: RwLock::new(Vec::with_capacity(capacity)),
            capacity,
        }
    }
    
    /// Record a completed trace
    pub async fn record_trace(&self, trace: TraceContext) {
        // Only record completed traces
        if !trace.completed || trace.total_ms.is_none() {
            warn!(
                trace_id = %trace.trace_id,
                "Attempted to record incomplete trace"
            );
            return;
        }
        
        let mut traces = self.traces.write().await;
        
        // If at capacity, remove oldest trace
        if traces.len() >= self.capacity {
            traces.remove(0);
        }
        
        // Add the new trace
        traces.push(trace);
    }
    
    /// Get recent traces
    pub async fn get_recent_traces(&self, limit: usize) -> Vec<TraceContext> {
        let traces = self.traces.read().await;
        let start_idx = if traces.len() > limit {
            traces.len() - limit
        } else {
            0
        };
        
        traces[start_idx..].to_vec()
    }
    
    /// Get a specific trace by ID
    pub async fn get_trace(&self, trace_id: Uuid) -> Option<TraceContext> {
        let traces = self.traces.read().await;
        traces.iter()
            .find(|t| t.trace_id == trace_id)
            .cloned()
    }
    
    /// Clear all traces
    pub async fn clear(&self) {
        let mut traces = self.traces.write().await;
        traces.clear();
    }
}

/// Global trace registry for collecting and analyzing traces
static TRACE_REGISTRY: std::sync::OnceLock<Arc<TraceRegistry>> = std::sync::OnceLock::new();

/// Get the global trace registry
pub fn get_trace_registry() -> Arc<TraceRegistry> {
    TRACE_REGISTRY.get_or_init(|| {
        // Default capacity of 1000 traces
        Arc::new(TraceRegistry::new(1000))
    }).clone()
}

#[cfg(test)]
mod tests {
    // Re-export tests from dedicated test file
    pub use crate::tests::flow_test::*;
}