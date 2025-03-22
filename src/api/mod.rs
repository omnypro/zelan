mod routes;
mod server;
mod handlers;

pub use server::start_server;

// Re-export HTTP server for use in main
pub use server::ApiServer;