pub mod base;
pub mod common; // New module for shared adapter code
pub mod http_client;
pub mod obs;
pub mod obs_callback;
pub mod test;
pub mod test_callback;
pub mod twitch;
pub mod twitch_api;
pub mod twitch_auth;
pub mod twitch_auth_callback;
pub mod twitch_eventsub;

// Tests module - only compiled in test mode
#[cfg(test)]
pub mod tests;

pub use base::{AdapterConfig, BaseAdapter};
pub use common::{AdapterError, BackoffStrategy, RetryOptions};
pub use http_client::{HttpClient, ReqwestHttpClient};
pub use obs::ObsAdapter;
pub use test::TestAdapter;
pub use twitch::TwitchAdapter;
