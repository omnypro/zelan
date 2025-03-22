pub mod adapters;
pub mod api;
pub mod auth;
pub mod core;
pub mod error;
pub mod service;
pub mod websocket;

// Re-export core components
pub use crate::adapters::{ServiceAdapter, ServiceAdapterStatus};
pub use crate::core::{EventBus, StreamEvent};
pub use crate::error::{Error, Result};
pub use crate::service::StreamService;
pub use crate::websocket::{WebSocketClientPreferences, WebSocketServer};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");