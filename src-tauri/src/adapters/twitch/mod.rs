//! Twitch adapter module

pub mod adapter;
pub mod api;
pub mod auth;
pub mod auth_callback;
pub mod eventsub;

// Re-export the main adapter components for backward compatibility
pub use adapter::{TwitchAdapter, TwitchConfig};
pub use auth::AuthEvent;
pub use auth_callback::TwitchAuthCallbackRegistry;
