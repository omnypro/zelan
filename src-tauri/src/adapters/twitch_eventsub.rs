use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::{env, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{mpsc, RwLock},
    time::timeout,
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};
use twitch_api::{
    eventsub::{event::websocket::EventsubWebsocketData, Event},
    types::UserId,
};
use twitch_oauth2::{ClientId, TwitchToken, UserToken};

use crate::adapters::common::{AdapterError, TraceHelper};
use crate::EventBus;
use crate::StreamEvent;

/// Default reconnect delay in seconds
pub(crate) const DEFAULT_RECONNECT_DELAY_SECS: u64 = 2;
/// Maximum reconnect delay in seconds (used for exponential backoff)
pub(crate) const MAX_RECONNECT_DELAY_SECS: u64 = 120;
/// WebSocket ping interval in seconds
const WEBSOCKET_PING_INTERVAL_SECS: u64 = 30;
/// WebSocket connection timeout in seconds
const WEBSOCKET_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Environment variable name for Twitch Client ID
const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

/// Get the Twitch Client ID from environment
fn get_client_id() -> Result<ClientId, AdapterError> {
    match env::var(TWITCH_CLIENT_ID_ENV) {
        Ok(client_id) if !client_id.is_empty() => Ok(ClientId::new(client_id)),
        Ok(_) => Err(AdapterError::config(
            "TWITCH_CLIENT_ID environment variable is empty",
        )),
        Err(_) => Err(AdapterError::config(
            "TWITCH_CLIENT_ID environment variable is not set",
        )),
    }
}

/// Represents a connection to Twitch EventSub WebSockets
#[derive(Debug)]
pub struct EventSubConnection {
    /// WebSocket connection
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    /// Session ID for this connection
    session_id: String,
    /// Last received keep-alive timestamp
    last_keepalive: std::time::Instant,
}

/// EventSub message types
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Session welcome message
    SessionWelcome,
    /// Keepalive message
    SessionKeepalive,
    /// Reconnect message (session will be terminated soon)
    SessionReconnect,
    /// Notification message (contains event data)
    Notification,
    /// Revocation message (subscription revoked)
    Revocation,
}

/// EventSub message payload
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventSubMessage {
    /// Message metadata
    pub metadata: EventSubMetadata,
    /// Message payload
    pub payload: EventSubPayload,
}

/// EventSub message metadata
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventSubMetadata {
    /// Message ID
    pub message_id: String,
    /// Message type
    pub message_type: MessageType,
    /// Message timestamp
    pub message_timestamp: String,
}

/// EventSub message payload
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventSubPayload {
    /// Session information (for welcome/reconnect)
    #[serde(default)]
    pub session: Option<EventSubSession>,
    /// Subscription information
    #[serde(default)]
    pub subscription: Option<EventSubSubscriptionInfo>,
    /// Event data
    #[serde(default)]
    pub event: Option<Value>,
}

/// EventSub session information
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventSubSession {
    /// Session ID
    pub id: String,
    /// Status
    pub status: String,
    /// Connected at timestamp
    #[serde(default)]
    pub connected_at: Option<String>,
    /// Keepalive timeout seconds
    #[serde(default)]
    pub keepalive_timeout_seconds: Option<u64>,
    /// Reconnect URL
    #[serde(default)]
    pub reconnect_url: Option<String>,
}

/// EventSub subscription information
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventSubSubscriptionInfo {
    /// Subscription ID
    pub id: String,
    /// Status
    pub status: String,
    /// Type
    #[serde(rename = "type")]
    pub subscription_type: String,
    /// Version
    pub version: String,
    /// Cost
    #[serde(default)]
    pub cost: Option<u64>,
    /// Condition
    pub condition: Value,
    /// Transport
    pub transport: Value,
    /// Created at timestamp
    pub created_at: String,
}

/// Callback function type for token refresh
pub type TokenRefresher = Arc<
    dyn Fn() -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<UserToken, anyhow::Error>> + Send>,
        > + Send
        + Sync,
>;

/// Twitch EventSub client
pub struct EventSubClient {
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Twitch Client ID
    client_id: ClientId,
    /// Active WebSocket connection - using Mutex for exclusive access
    connection: Arc<tokio::sync::Mutex<Option<EventSubConnection>>>,
    /// Shutdown signal
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    /// User ID (broadcaster ID)
    user_id: Arc<RwLock<Option<String>>>,
    /// Current auth token
    token: Arc<RwLock<Option<UserToken>>>,
    /// Token refresher callback
    token_refresher: Arc<RwLock<Option<TokenRefresher>>>,
    /// Last token hash for detecting changes
    token_hash: Arc<RwLock<Option<u64>>>,
}

impl Clone for EventSubClient {
    fn clone(&self) -> Self {
        // IMPORTANT: All fields that need to maintain state consistency
        // are already wrapped in Arc, so we just need to clone those Arcs
        // to preserve shared state, especially callbacks like token_refresher
        Self {
            event_bus: Arc::clone(&self.event_bus),
            client_id: self.client_id.clone(),
            connection: Arc::clone(&self.connection),
            shutdown_tx: Arc::clone(&self.shutdown_tx),
            user_id: Arc::clone(&self.user_id),
            token: Arc::clone(&self.token),
            token_refresher: Arc::clone(&self.token_refresher),
            token_hash: Arc::clone(&self.token_hash),
        }
    }
}

// We're no longer parsing chat messages and just passing through the raw data

impl EventSubClient {
    /// Create a new EventSub client
    pub async fn new(event_bus: Arc<EventBus>) -> Result<Self, AdapterError> {
        // Record the operation in the trace system
        TraceHelper::record_adapter_operation("twitch_eventsub", "client_creation", None).await;

        // Get client ID from environment
        let client_id = get_client_id()?;

        let client = Self {
            event_bus,
            client_id,
            connection: Arc::new(tokio::sync::Mutex::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            token: Arc::new(RwLock::new(None)),
            token_refresher: Arc::new(RwLock::new(None)),
            token_hash: Arc::new(RwLock::new(None)),
        };

        // Record successful creation
        TraceHelper::record_adapter_operation("twitch_eventsub", "client_created_success", None)
            .await;

        Ok(client)
    }

    /// Set the token refresher callback
    pub fn set_token_refresher(&self, refresher: TokenRefresher) {
        let token_refresher = self.token_refresher.clone();
        tokio::spawn(async move {
            *token_refresher.write().await = Some(refresher);
        });
    }

    /// Update the token stored in the client
    pub async fn update_token(&self, token: UserToken) -> Result<(), AdapterError> {
        // Record the operation in the trace system
        TraceHelper::record_adapter_operation("twitch_eventsub", "token_update_start", None).await;

        // Calculate token hash for change detection
        let token_hash = self.hash_token(&token);

        // Store token and its hash
        *self.token.write().await = Some(token);
        *self.token_hash.write().await = Some(token_hash);

        // Record successful update
        TraceHelper::record_adapter_operation("twitch_eventsub", "token_update_success", None)
            .await;

        Ok(())
    }

    /// Get a fresh token (refresh if needed)
    pub async fn get_fresh_token(&self) -> Result<UserToken, AdapterError> {
        // Record the operation in the trace system
        TraceHelper::record_adapter_operation("twitch_eventsub", "get_fresh_token_start", None)
            .await;

        // First check if we need to refresh
        if let Err(e) = self.check_and_refresh_token_if_needed().await {
            error!("Error refreshing token during get_fresh_token: {}", e);

            // Record the error
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                "token_refresh_error",
                Some(serde_json::json!({
                    "error": e.to_string(),
                })),
            )
            .await;

            // Continue anyway and try to use what we have
        }

        // Get the current token
        let token_guard = self.token.read().await;
        match &*token_guard {
            Some(token) => {
                // Record successful token access
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "get_fresh_token_success",
                    Some(serde_json::json!({
                        "expires_in_seconds": token.expires_in().as_secs(),
                    })),
                )
                .await;

                Ok(token.clone())
            }
            None => {
                let error = AdapterError::auth("No token available");

                // Record the error
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "get_fresh_token_error",
                    Some(serde_json::json!({
                        "error": error.to_string(),
                    })),
                )
                .await;

                Err(error)
            }
        }
    }

    /// Check if token needs refresh and refresh it if needed
    pub async fn check_and_refresh_token_if_needed(&self) -> Result<(), AdapterError> {
        // Record the operation start
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "check_token_expiration_start",
            None,
        )
        .await;

        // Get current token
        let current_token = {
            let token_guard = self.token.read().await;
            match &*token_guard {
                Some(token) => token.clone(),
                None => {
                    let error = AdapterError::auth("No token available to refresh");

                    // Record the error
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        "check_token_expiration_error",
                        Some(serde_json::json!({
                            "error": error.to_string(),
                        })),
                    )
                    .await;

                    return Err(error);
                }
            }
        };

        // Check if token is about to expire (within 10 minutes)
        let expires_in = current_token.expires_in();
        if expires_in.as_secs() < 600 {
            info!(
                "Token expires in {} seconds, refreshing",
                expires_in.as_secs()
            );

            // Record that token needs refresh
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                "token_needs_refresh",
                Some(serde_json::json!({
                    "expires_in_seconds": expires_in.as_secs(),
                })),
            )
            .await;

            // Get the refresher function
            let refresher = {
                let refresher_guard = self.token_refresher.read().await;
                match &*refresher_guard {
                    Some(refresher) => refresher.clone(),
                    None => {
                        let error = AdapterError::auth("No token refresher available");

                        // Record the error
                        TraceHelper::record_adapter_operation(
                            "twitch_eventsub",
                            "token_refresh_error",
                            Some(serde_json::json!({
                                "error": error.to_string(),
                            })),
                        )
                        .await;

                        return Err(error);
                    }
                }
            };

            // Use our new retry helper for token refreshing
            use crate::common::retry::{linear_backoff, with_jitter, with_retry_and_backoff};

            // Set up operation name and clone refresher for retry usage
            let operation_name = "token_refresh";
            let refresher_clone = refresher.clone();

            // Start tracing the operation
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                &format!("{}_start", operation_name),
                Some(serde_json::json!({
                    "max_attempts": 2, // Using 2 attempts like before
                    "operation": operation_name,
                })),
            )
            .await;

            // Create a linear backoff strategy with jitter
            let backoff_fn = with_jitter(linear_backoff(500));

            // Use the retry helper with our custom backoff strategy
            let new_token = with_retry_and_backoff(
                || {
                    // Clone the closure to satisfy borrowing requirements
                    let refresher = refresher_clone.clone();

                    Box::pin(async move {
                        // Record attempt
                        TraceHelper::record_adapter_operation(
                            "twitch_eventsub",
                            &format!("{}_attempt", operation_name),
                            Some(serde_json::json!({
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        debug!("Attempting to refresh token");

                        // Execute the token refresh operation
                        match refresher().await {
                            Ok(token) => Ok(token),
                            Err(e) => {
                                // Convert anyhow error to AdapterError
                                let error = AdapterError::from_anyhow_error(
                                    "auth",
                                    "Failed to refresh token",
                                    e,
                                );

                                // Record failure
                                TraceHelper::record_adapter_operation(
                                    "twitch_eventsub",
                                    &format!("{}_failure", operation_name),
                                    Some(serde_json::json!({
                                        "error": error.to_string(),
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;

                                Err(error)
                            }
                        }
                    })
                },
                2, // max attempts (same as before)
                operation_name,
                backoff_fn,
            )
            .await?;

            // Record success
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                &format!("{}_success", operation_name),
                Some(serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            // Calculate token hash for the new token
            let token_hash = self.hash_token(&new_token);
            let current_hash = {
                let hash_guard = self.token_hash.read().await;
                hash_guard.clone()
            };

            // Update token if hash is different (token has changed)
            if current_hash.is_none() || current_hash.unwrap() != token_hash {
                info!("Token has changed, updating stored token");

                // Record token change
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "token_changed",
                    Some(serde_json::json!({
                        "expires_in_seconds": new_token.expires_in().as_secs(),
                    })),
                )
                .await;

                self.update_token(new_token).await?;
            } else {
                info!("Token is unchanged after refresh");

                // Record no change
                TraceHelper::record_adapter_operation("twitch_eventsub", "token_unchanged", None)
                    .await;
            }

            Ok(())
        } else {
            // Token is still valid
            debug!("Token is still valid for {} seconds", expires_in.as_secs());

            // Record that token is still valid
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                "token_still_valid",
                Some(serde_json::json!({
                    "expires_in_seconds": expires_in.as_secs(),
                })),
            )
            .await;

            Ok(())
        }
    }

    /// Calculate a hash for the token to detect changes
    fn hash_token(&self, token: &UserToken) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        token.access_token.secret().hash(&mut hasher);
        if let Some(refresh_token) = &token.refresh_token {
            refresh_token.secret().hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Start the EventSub client
    pub async fn start(&self, token: &UserToken) -> Result<(), AdapterError> {
        // Record the operation start
        TraceHelper::record_adapter_operation("twitch_eventsub", "client_start", None).await;

        info!("Starting Twitch EventSub client");

        // Try to get a lock on the connection to check if it already exists
        if let Ok(guard) = self.connection.try_lock() {
            if guard.is_some() {
                info!("EventSub client already has a connection, skipping start");

                // Record already started
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "client_already_started",
                    None,
                )
                .await;

                return Ok(());
            }
            // Release the lock immediately
            drop(guard);
        } else {
            // Couldn't get the lock, which means someone else is working with it
            info!("EventSub client is already being modified by another task, skipping start");

            // Record lock conflict
            TraceHelper::record_adapter_operation(
                "twitch_eventsub",
                "client_start_lock_conflict",
                None,
            )
            .await;

            return Ok(());
        }

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Store the initial token
        self.update_token(token.clone()).await?;

        // Get broadcaster ID from token user ID
        let user_id = Self::get_user_id_from_token(&self.client_id, token).await?;
        *self.user_id.write().await = Some(user_id.to_string());

        // Record user ID obtained
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "user_id_obtained",
            Some(serde_json::json!({
                "user_id": user_id.to_string(),
            })),
        )
        .await;

        // Clone Arc references for the background task
        let event_bus = self.event_bus.clone();
        let connection = self.connection.clone();
        let client_id = self.client_id.clone();
        let token_storage = self.token.clone();
        let token_refresher = self.token_refresher.clone();
        let user_id_clone = self.user_id.clone();

        // Spawn the WebSocket connection task
        tokio::spawn(async move {
            // Track reconnect attempts for exponential backoff
            let mut reconnect_attempts = 0;
            // Create a periodic token check timer (every 5 minutes)
            let mut token_check_interval = tokio::time::interval(Duration::from_secs(300));
            // First tick completes immediately, subsequent ticks wait for the interval
            token_check_interval.tick().await;

            // Connection loop
            loop {
                // Check for shutdown signal
                if let Ok(Some(_)) = timeout(Duration::from_millis(10), shutdown_rx.recv()).await {
                    info!("Shutdown signal received, stopping EventSub client");
                    break;
                }

                // Periodic token check (non-blocking)
                if tokio::time::timeout(Duration::from_millis(1), token_check_interval.tick())
                    .await
                    .is_ok()
                {
                    // Token check interval reached

                    // Get the current token
                    match token_storage.read().await.clone() {
                        Some(current_token) => {
                            // Check if token is expiring soon (within 10 minutes)
                            let expires_in = current_token.expires_in();
                            if expires_in.as_secs() < 600 {
                                info!(
                                    "Token expiring in {} seconds, attempting refresh",
                                    expires_in.as_secs()
                                );

                                // Check if we have a refresher
                                if let Some(refresher) = token_refresher.read().await.clone() {
                                    // Try to refresh token
                                    match refresher().await {
                                        Ok(new_token) => {
                                            info!("Token successfully refreshed, expires in {} seconds", 
                                                  new_token.expires_in().as_secs());
                                            // Update stored token
                                            *token_storage.write().await = Some(new_token);

                                            // Force reconnection to use the new token
                                            let mut conn = connection.lock().await;
                                            if conn.is_some() {
                                                info!("Reconnecting to use the new token");
                                                *conn = None;
                                                drop(conn);

                                                // Reset reconnect attempts since this is intentional
                                                reconnect_attempts = 0;
                                                continue;
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to refresh token: {}", e);
                                            // Continue anyway, will reconnect on 401
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            warn!("No token available for periodic check");
                        }
                    }
                }

                // Calculate reconnect delay with exponential backoff using our new retry helper
                use crate::common::retry::{exponential_backoff, with_jitter};

                // No delay for first attempt
                let reconnect_delay = if reconnect_attempts == 0 {
                    Duration::ZERO
                } else {
                    // Create an exponential backoff with jitter function
                    let backoff_fn = with_jitter(exponential_backoff(
                        DEFAULT_RECONNECT_DELAY_SECS * 1000,   // base delay in ms
                        Some(MAX_RECONNECT_DELAY_SECS * 1000), // max delay in ms
                    ));

                    // Calculate delay based on attempt number
                    backoff_fn(reconnect_attempts)
                };

                // Wait for reconnect delay if needed
                if !reconnect_delay.is_zero() {
                    info!(
                        "Reconnecting to Twitch EventSub in {:.2} seconds",
                        reconnect_delay.as_secs_f64()
                    );
                    tokio::time::sleep(reconnect_delay).await;
                }

                // Connect to EventSub WebSocket
                let result = Self::connect_to_eventsub(&connection).await;

                match result {
                    Ok(session_id) => {
                        // Connection successful, reset reconnect attempts
                        reconnect_attempts = 0;
                        info!("Connected to Twitch EventSub, session_id: {}", session_id);

                        // Create subscriptions immediately after connecting
                        // This must happen within 10 seconds or Twitch will close the connection with code 4003
                        let broadcaster_id = match user_id_clone.read().await.clone() {
                            Some(id) => id,
                            None => {
                                error!("No broadcaster ID available for creating EventSub subscriptions");
                                break;
                            }
                        };

                        info!(
                            "Creating EventSub subscriptions for broadcaster ID: {}",
                            broadcaster_id
                        );

                        // Get the most up-to-date token for creating subscriptions
                        let current_token = match token_storage.read().await.clone() {
                            Some(token) => token,
                            None => {
                                error!("No token available for creating EventSub subscriptions");
                                break;
                            }
                        };

                        // Create subscriptions for various event types
                        match Self::create_subscriptions(
                            &client_id,
                            &current_token,
                            &broadcaster_id,
                            &session_id,
                        )
                        .await
                        {
                            Ok(count) => {
                                info!("Successfully created {} EventSub subscriptions", count);
                            }
                            Err(e) => {
                                error!("Failed to create EventSub subscriptions: {}", e);

                                // Check if the error is due to authentication (401)
                                if e.to_string().contains("401")
                                    || e.to_string().contains("Authentication")
                                {
                                    info!("Authentication error during subscription creation, will attempt token refresh");

                                    // Try to refresh the token
                                    if let Some(refresher) = token_refresher.read().await.clone() {
                                        match refresher().await {
                                            Ok(new_token) => {
                                                info!("Token refreshed after 401 error");
                                                *token_storage.write().await = Some(new_token);
                                                // Force reconnection loop
                                                break;
                                            }
                                            Err(refresh_err) => {
                                                error!(
                                                    "Failed to refresh token after 401: {}",
                                                    refresh_err
                                                );
                                            }
                                        }
                                    }
                                }

                                // Continue anyway, we might handle some predefined subscriptions
                            }
                        }

                        // Process messages until disconnected
                        Self::process_messages(
                            &connection,
                            &event_bus,
                            &user_id_clone,
                            &token_storage,
                            &token_refresher,
                        )
                        .await;
                    }
                    Err(e) => {
                        // Connection failed
                        error!("Failed to connect to Twitch EventSub: {}", e);
                        reconnect_attempts += 1;
                    }
                }

                // Clear connection before reconnecting
                if let Ok(mut conn) = connection.try_lock() {
                    *conn = None;
                }
            }

            // Clear connection on shutdown
            if let Ok(mut conn) = connection.try_lock() {
                *conn = None;
            }
            info!("EventSub client stopped");
        });

        Ok(())
    }

    /// Stop the EventSub client
    pub async fn stop(&self) -> Result<(), AdapterError> {
        // Record the operation start
        TraceHelper::record_adapter_operation("twitch_eventsub", "client_stop", None).await;

        info!("Stopping Twitch EventSub client");

        // Send shutdown signal first
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            if let Err(e) = tx.send(()).await {
                // Non-fatal error, as the receiver might be already dropped
                warn!("Failed to send shutdown signal: {}", e);

                // Record shutdown signal error
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "shutdown_signal_error",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                    })),
                )
                .await;
            } else {
                // Record shutdown signal sent
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "shutdown_signal_sent",
                    None,
                )
                .await;
            }
        }

        // Clear the connection
        let mut conn = self.connection.lock().await;
        *conn = None;
        info!("Successfully cleared EventSub connection");

        // Record successful stop
        TraceHelper::record_adapter_operation("twitch_eventsub", "client_stop_success", None).await;

        Ok(())
    }

    /// Check if the client is currently connected
    pub fn is_connected(&self) -> bool {
        // Since we can't easily await in this context, we'll use try_lock
        // If we can't get the lock, we assume the connection is in use (likely connected)
        if let Ok(conn) = self.connection.try_lock() {
            // If we got the lock, check if the connection exists
            conn.is_some()
        } else {
            // If we couldn't get the lock, the connection is being used,
            // which generally means it's connected or in the process of connecting
            true
        }
    }

    /// Connect to Twitch EventSub WebSocket
    async fn connect_to_eventsub(
        connection: &Arc<tokio::sync::Mutex<Option<EventSubConnection>>>,
    ) -> Result<String, AdapterError> {
        // Record the operation in the trace system
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "connect_start",
            Some(serde_json::json!({
                "websocket_url": "wss://eventsub.wss.twitch.tv/ws",
                "timeout_seconds": WEBSOCKET_CONNECT_TIMEOUT_SECS,
            })),
        )
        .await;

        // Get an exclusive lock on the connection for the duration of this function
        // This ensures only one task can create a connection at a time
        let mut conn_lock = connection.lock().await;

        // First check if there's already a valid connection we can reuse
        if let Some(existing_conn) = &*conn_lock {
            if !existing_conn.session_id.is_empty() {
                info!(
                    "Reusing existing EventSub connection with session_id: {}",
                    existing_conn.session_id
                );
                // Record successful reuse in trace
                TraceHelper::record_adapter_operation(
                    "twitch_eventsub",
                    "connection_reused",
                    Some(serde_json::json!({
                        "session_id": existing_conn.session_id,
                    })),
                )
                .await;
                return Ok(existing_conn.session_id.clone());
            }
        }

        // If we reach here, we need to create a new connection
        info!("Connecting to Twitch EventSub WebSocket");

        // EventSub WebSocket URL
        let ws_url = "wss://eventsub.wss.twitch.tv/ws";

        // Use our new retry helper
        use crate::common::retry::with_retry;

        // Set up operation name for tracing/logging
        let operation_name = "connect_to_eventsub";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": 3,
                "websocket_url": ws_url,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Use the retry helper for WebSocket connection
        let ws_stream = with_retry(
            || {
                let ws_url = ws_url.to_string();

                Box::pin(async move {
                    // Record attempt
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        &format!("{}_attempt", operation_name),
                        Some(serde_json::json!({
                            "websocket_url": ws_url,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    debug!("Attempting to connect to EventSub WebSocket");

                    // Execute the operation with timeout
                    // Using match instead of map_err to allow await in error handling
                    let connect_result = match timeout(
                        Duration::from_secs(WEBSOCKET_CONNECT_TIMEOUT_SECS),
                        connect_async(&ws_url),
                    ).await {
                        Ok(result) => result,
                        Err(_) => {
                            let error = AdapterError::connection(format!(
                                "WebSocket connection timed out after {}s",
                                WEBSOCKET_CONNECT_TIMEOUT_SECS
                            ));

                            // Record timeout
                            TraceHelper::record_adapter_operation(
                                "twitch_eventsub",
                                &format!("{}_timeout", operation_name),
                                Some(serde_json::json!({
                                    "timeout_seconds": WEBSOCKET_CONNECT_TIMEOUT_SECS,
                                    "websocket_url": ws_url,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Handle connection result - using match instead of map_err
                    let (ws_stream, _) = match connect_result {
                        Ok(ws) => ws,
                        Err(e) => {
                            let error = AdapterError::connection_with_source(
                                format!("WebSocket connection failed: {}", e),
                                e,
                            );

                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch_eventsub",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "websocket_url": ws_url,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Success! Record and return the stream
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        &format!("{}_success", operation_name),
                        Some(serde_json::json!({
                            "websocket_url": ws_url,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    Ok(ws_stream)
                })
            },
            3, // max attempts
            operation_name,
        )
        .await?;

        // Create connection with no session ID yet
        let mut conn = EventSubConnection {
            ws_stream,  // ws_stream is already unwrapped by the retry function
            session_id: String::new(),
            last_keepalive: std::time::Instant::now(),
        };

        // We'll keep the lock while we wait for the welcome message
        // Record waiting for welcome message in trace
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "waiting_for_welcome",
            Some(serde_json::json!({
                "timeout_seconds": 10,
            })),
        )
        .await;

        // Wait for welcome message to get session ID
        let welcome_timeout = Duration::from_secs(10);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < welcome_timeout {
            // Try to receive a message
            match timeout(Duration::from_secs(1), conn.ws_stream.next()).await {
                Ok(Some(Ok(msg))) => {
                    if let WsMessage::Text(text) = msg {
                        // Try to parse using the twitch_api event parser
                        match Event::parse_websocket(&text) {
                            Ok(EventsubWebsocketData::Welcome { payload, .. }) => {
                                // Extract session ID
                                let session_id = payload.session.id.into_owned();

                                // Store session ID
                                conn.session_id = session_id.clone();

                                // Store the connection
                                *conn_lock = Some(conn);

                                info!("Successfully stored new EventSub connection with session_id: {}", session_id);

                                // Record successful connection in trace
                                TraceHelper::record_adapter_operation(
                                    "twitch_eventsub",
                                    "connect_success",
                                    Some(serde_json::json!({
                                        "session_id": session_id,
                                    })),
                                )
                                .await;

                                return Ok(session_id);
                            }
                            Ok(_) => {
                                // Unexpected message type - continue waiting for welcome
                                warn!("Received non-welcome message during connection");
                            }
                            Err(e) => {
                                warn!("Failed to parse EventSub message: {}", e);
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let error = AdapterError::connection_with_source(
                        format!("WebSocket error while waiting for welcome: {}", e),
                        e,
                    );

                    // Record error in trace
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        "connect_error",
                        Some(serde_json::json!({
                            "error": error.to_string(),
                        })),
                    )
                    .await;

                    return Err(error);
                }
                Ok(None) => {
                    let error = AdapterError::connection(
                        "WebSocket closed unexpectedly while waiting for welcome",
                    );

                    // Record error in trace
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        "connect_error",
                        Some(serde_json::json!({
                            "error": error.to_string(),
                        })),
                    )
                    .await;

                    return Err(error);
                }
                Err(_) => {
                    // Timeout on this iteration, continue
                    continue;
                }
            }
        }

        let error = AdapterError::connection("Timed out waiting for welcome message");

        // Record error in trace
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "connect_timeout",
            Some(serde_json::json!({
                "error": error.to_string(),
                "timeout_seconds": welcome_timeout.as_secs(),
            })),
        )
        .await;

        Err(error)
    }

    /// Get user ID from token
    async fn get_user_id_from_token(
        client_id: &ClientId,
        token: &UserToken,
    ) -> Result<UserId, AdapterError> {
        // Record the operation in the trace system
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            "get_user_id_start",
            Some(serde_json::json!({
                "endpoint": "https://api.twitch.tv/helix/users",
            })),
        )
        .await;

        info!("Getting user ID from token");

        // Use our new retry helper
        use crate::common::retry::with_retry;

        // Set up operation name for logging and tracing
        let operation_name = "get_user_id_from_token";

        // Start tracing the operation
        TraceHelper::record_adapter_operation(
            "twitch_eventsub",
            &format!("{}_start", operation_name),
            Some(serde_json::json!({
                "max_attempts": 3,
                "endpoint": "https://api.twitch.tv/helix/users",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Clone values needed for the retry operation
        let client_id_clone = client_id.clone();
        let token_clone = token.clone();

        // Use our retry helper
        with_retry(
            || {
                // Clone values for use in the async block
                let client_id = client_id_clone.clone();
                let token = token_clone.clone();

                Box::pin(async move {
                    // Record attempt
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        &format!("{}_attempt", operation_name),
                        Some(serde_json::json!({
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    debug!("Attempting to get user ID from token");

                    // Create client and make the request - using match instead of map_err
                    let client = reqwest::Client::new();
                    let response = match client
                        .get("https://api.twitch.tv/helix/users")
                        .header("Client-ID", client_id.as_str())
                        .header(
                            "Authorization",
                            format!("Bearer {}", token.access_token.secret()),
                        )
                        .send()
                        .await 
                    {
                        Ok(resp) => resp,
                        Err(e) => {
                            let error = AdapterError::api_with_source(
                                format!("Failed to send request to Twitch API: {}", e),
                                e,
                            );

                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch_eventsub",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Check response status
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|e| format!("Failed to read error response: {}", e));

                        let error = AdapterError::api_with_status(
                            format!("Failed to get user info: HTTP {} - {}", status, error_text),
                            status.as_u16(),
                        );

                        // Record failure
                        TraceHelper::record_adapter_operation(
                            "twitch_eventsub",
                            &format!("{}_failure", operation_name),
                            Some(serde_json::json!({
                                "status_code": status.as_u16(),
                                "error": error.to_string(),
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        )
                        .await;

                        return Err(error);
                    }

                    // Parse response - using match instead of map_err
                    let response_json: Value = match response.json().await {
                        Ok(json) => json,
                        Err(e) => {
                            let error = AdapterError::api_with_source(
                                format!("Failed to parse response JSON: {}", e),
                                e,
                            );

                            // Record failure
                            TraceHelper::record_adapter_operation(
                                "twitch_eventsub",
                                &format!("{}_failure", operation_name),
                                Some(serde_json::json!({
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            return Err(error);
                        }
                    };

                    // Extract user ID
                    if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
                        if let Some(user) = data.first() {
                            if let Some(id) = user.get("id").and_then(|id| id.as_str()) {
                                // Success! Record and return the ID
                                TraceHelper::record_adapter_operation(
                                    "twitch_eventsub",
                                    &format!("{}_success", operation_name),
                                    Some(serde_json::json!({
                                        "user_id": id,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;

                                return Ok(UserId::new(id.to_string()));
                            }
                        }
                    }

                    // If we get here, couldn't extract user ID
                    let error = AdapterError::api("Failed to extract user ID from response");

                    // Record failure
                    TraceHelper::record_adapter_operation(
                        "twitch_eventsub",
                        &format!("{}_failure", operation_name),
                        Some(serde_json::json!({
                            "error": error.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    Err(error)
                })
            },
            3, // max attempts (same as before)
            operation_name,
        )
        .await
    }

    /// Create EventSub subscriptions using create_eventsub_subscription() from the library
    async fn create_subscriptions(
        _client_id: &ClientId,
        token: &UserToken,
        broadcaster_id: &str,
        session_id: &str,
    ) -> Result<usize, AdapterError> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use twitch_api::{
            eventsub::{
                channel::{
                    ChannelBanV1, ChannelChatMessageV1, ChannelCheerV1, ChannelFollowV2,
                    ChannelModeratorAddV1, ChannelModeratorRemoveV1, ChannelRaidV1,
                    ChannelSubscribeV1, ChannelSubscriptionEndV1, ChannelSubscriptionGiftV1,
                    ChannelSubscriptionMessageV1, ChannelUnbanV1, ChannelUpdateV2,
                },
                stream::{StreamOfflineV1, StreamOnlineV1},
            },
            helix::HelixClient,
        };

        // Parse the broadcaster ID string into a UserId
        let broadcaster = UserId::new(broadcaster_id.to_string());
        let success_count = Arc::new(AtomicUsize::new(0));

        // Create a reqwest HTTP client for the Helix client
        let reqw_client = reqwest::Client::new();
        // Create a Helix client with the HTTP client
        let helix_client: HelixClient<reqwest::Client> = HelixClient::with_client(reqw_client);

        // Create the websocket transport for the session
        let transport = twitch_api::eventsub::Transport::websocket(session_id);

        // Helper function to create a subscription with error handling and delay
        async fn create_sub<C>(
            name: &str,
            helix_client: &HelixClient<'_, reqwest::Client>,
            condition: C,
            transport: &twitch_api::eventsub::Transport,
            token: &UserToken,
            success_count: Arc<AtomicUsize>,
        ) -> Result<(), AdapterError>
        where
            C: twitch_api::eventsub::EventSubscription + std::fmt::Debug + Send,
        {
            info!("Creating subscription for {}", name);
            match helix_client
                .create_eventsub_subscription(condition, transport.clone(), token)
                .await
            {
                Ok(_) => {
                    info!("Successfully created subscription for {}", name);
                    success_count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to create subscription for {}: {}", name, e);
                    Err(AdapterError::api_with_source(
                        format!("Failed to create subscription for {}", name),
                        e,
                    ))
                }
            }
        }

        // Vector of (name, subscription creation function) pairs
        let subscriptions: Vec<(
            &str,
            Box<
                dyn FnOnce() -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<(), AdapterError>> + Send>,
                    > + Send,
            >,
        )> = vec![
            // Stream events
            (
                "stream.online",
                Box::new(|| {
                    Box::pin(create_sub(
                        "stream.online",
                        &helix_client,
                        StreamOnlineV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "stream.offline",
                Box::new(|| {
                    Box::pin(create_sub(
                        "stream.offline",
                        &helix_client,
                        StreamOfflineV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            // Channel events
            (
                "channel.update",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.update",
                        &helix_client,
                        ChannelUpdateV2::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.follow",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.follow",
                        &helix_client,
                        ChannelFollowV2::new(broadcaster.clone(), broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.subscribe",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.subscribe",
                        &helix_client,
                        ChannelSubscribeV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.subscription.end",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.subscription.end",
                        &helix_client,
                        ChannelSubscriptionEndV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.subscription.gift",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.subscription.gift",
                        &helix_client,
                        ChannelSubscriptionGiftV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.subscription.message",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.subscription.message",
                        &helix_client,
                        ChannelSubscriptionMessageV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.cheer",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.cheer",
                        &helix_client,
                        ChannelCheerV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.raid",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.raid",
                        &helix_client,
                        ChannelRaidV1::to_broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.ban",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.ban",
                        &helix_client,
                        ChannelBanV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.unban",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.unban",
                        &helix_client,
                        ChannelUnbanV1::broadcaster_user_id(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.moderator.add",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.moderator.add",
                        &helix_client,
                        ChannelModeratorAddV1::new(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            (
                "channel.moderator.remove",
                Box::new(|| {
                    Box::pin(create_sub(
                        "channel.moderator.remove",
                        &helix_client,
                        ChannelModeratorRemoveV1::new(broadcaster.clone()),
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
            // Chat message event
            (
                "channel.chat.message",
                Box::new(|| {
                    let chat_condition = ChannelChatMessageV1::new(
                        broadcaster.clone(),
                        broadcaster.clone(), // Using broadcaster ID for chatter too to match any user
                    );
                    Box::pin(create_sub(
                        "channel.chat.message",
                        &helix_client,
                        chat_condition,
                        &transport,
                        token,
                        success_count.clone(),
                    ))
                }),
            ),
        ];

        // Create subscriptions for all event types
        info!(
            "Creating EventSub subscriptions for broadcaster {}",
            broadcaster_id
        );

        let mut successful = 0;

        for (name, create_sub_fn) in subscriptions {
            // Create the subscription
            match create_sub_fn().await {
                Ok(_) => {
                    successful += 1;
                    info!("Created subscription for {}", name);
                }
                Err(e) => {
                    error!("Failed to create subscription for {}: {}", name, e);
                    // If the session is disconnected, stop trying to create more
                    if e.to_string().contains("disconnected") {
                        error!("WebSocket session disconnected - subscriptions will be created on reconnection");
                        break;
                    }
                }
            }

            // Add a small delay between requests to respect rate limits
            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        info!("Successfully created {} EventSub subscriptions", successful);

        // Return the count of successfully created subscriptions
        Ok(successful)
    }

    /// Process EventSub WebSocket messages
    async fn process_messages(
        connection: &Arc<tokio::sync::Mutex<Option<EventSubConnection>>>,
        event_bus: &Arc<EventBus>,
        user_id: &Arc<RwLock<Option<String>>>,
        token_storage: &Arc<RwLock<Option<UserToken>>>,
        token_refresher: &Arc<RwLock<Option<TokenRefresher>>>,
    ) {
        info!("Started processing EventSub messages");

        // Create ping timer
        let mut ping_interval =
            tokio::time::interval(Duration::from_secs(WEBSOCKET_PING_INTERVAL_SECS));

        // Set up a task to send periodic pings to keep the connection alive
        let connection_ping = connection.clone();
        let ping_task = tokio::spawn(async move {
            loop {
                ping_interval.tick().await;

                // Try to acquire lock without blocking - if we can't get it now, we'll try next tick
                match connection_ping.try_lock() {
                    Ok(mut conn_guard) => {
                        // If we have a connection, send a ping
                        if let Some(conn) = &mut *conn_guard {
                            let elapsed = conn.last_keepalive.elapsed();
                            debug!("Last keepalive: {:?} ago", elapsed);

                            if let Err(e) =
                                conn.ws_stream.send(WsMessage::Ping(vec![].into())).await
                            {
                                error!("Failed to send ping: {}", e);
                                break;
                            }
                        } else {
                            // No connection means we're done
                            break;
                        }
                    }
                    Err(_) => {
                        // Couldn't get the lock, try again next tick
                        continue;
                    }
                }
            }
            debug!("Ping task stopped");
        });

        // Message processing loop
        loop {
            // Get a lock on the connection
            let mut conn_guard = connection.lock().await;

            // If connection is None, we're done
            let conn = match &mut *conn_guard {
                Some(c) => c,
                None => {
                    debug!("Connection closed, stopping message processing");
                    break;
                }
            };

            // Try to receive a message
            match timeout(Duration::from_secs(1), conn.ws_stream.next()).await {
                Ok(Some(Ok(msg))) => {
                    match msg {
                        WsMessage::Text(text) => {
                            // Try to parse using the twitch_api event parser
                            match Event::parse_websocket(&text) {
                                Ok(websocket_data) => {
                                    // Process based on message type
                                    match websocket_data {
                                        EventsubWebsocketData::Keepalive { .. } => {
                                            debug!("Received keepalive");
                                            conn.last_keepalive = std::time::Instant::now();
                                        }
                                        EventsubWebsocketData::Reconnect { payload, .. } => {
                                            if let Some(reconnect_url) =
                                                &payload.session.reconnect_url
                                            {
                                                info!(
                                                    "Received reconnect message, url: {}",
                                                    reconnect_url
                                                );

                                                // We'll drop the connection and let the main loop reconnect
                                                drop(conn_guard);
                                                break;
                                            }
                                        }
                                        EventsubWebsocketData::Notification {
                                            payload,
                                            metadata,
                                        } => {
                                            // Process the event
                                            info!(
                                                "Received notification: {} ({})",
                                                metadata.message_id, metadata.subscription_type
                                            );

                                            // Convert to JSON for our simple EventBus
                                            match serde_json::to_value(&payload) {
                                                Ok(event_json) => {
                                                    // broadcaster_id is retrieved but not used in this function
                                                    // keeping for future implementation that might need it
                                                    let _broadcaster_id =
                                                        user_id.read().await.clone();

                                                    // Map the event type to our internal format
                                                    let event_type = match metadata
                                                        .subscription_type
                                                        .to_str()
                                                    {
                                                        // Stream events
                                                        "stream.online" => "stream.online",
                                                        "stream.offline" => "stream.offline",

                                                        // Channel events
                                                        "channel.update" => "channel.updated",
                                                        "channel.follow" => "channel.follow",
                                                        "channel.subscribe" => "channel.subscribe",
                                                        "channel.subscription.end" => {
                                                            "channel.subscription.end"
                                                        }
                                                        "channel.subscription.gift" => {
                                                            "channel.subscription.gift"
                                                        }
                                                        "channel.subscription.message" => {
                                                            "channel.subscription.message"
                                                        }
                                                        "channel.cheer" => "channel.cheer",
                                                        "channel.raid" => "channel.raid",
                                                        "channel.ban" => "channel.ban",
                                                        "channel.unban" => "channel.unban",
                                                        "channel.moderator.add" => {
                                                            "channel.moderator.add"
                                                        }
                                                        "channel.moderator.remove" => {
                                                            "channel.moderator.remove"
                                                        }

                                                        // Chat message events
                                                        "channel.chat.message" => "chat.message",

                                                        // Unknown event type
                                                        _ => {
                                                            warn!(
                                                                "Unhandled event type: {}",
                                                                metadata.subscription_type
                                                            );
                                                            continue;
                                                        }
                                                    };

                                                    // Extract data specifically targeting the known Twitch API struct names
                                                    let extract_by_struct_name = |json: &Value, struct_name: &str| -> Value {
                                                        // Directly extract the data from the named struct
                                                        if let Some(payload) = json.get(struct_name) {
                                                            return payload.clone();
                                                        }
                                                        json.clone()
                                                    };

                                                    // Map the event type to the corresponding struct name from the Twitch API
                                                    let struct_name = match event_type {
                                                        "stream.online" => "StreamOnlineV1",
                                                        "stream.offline" => "StreamOfflineV1",
                                                        "channel.updated" => "ChannelUpdateV2",
                                                        "chat.message" => "ChannelChatMessageV1",
                                                        "channel.follow" => "ChannelFollowV2",
                                                        "channel.subscribe" => "ChannelSubscribeV1",
                                                        "channel.subscription.end" => {
                                                            "ChannelSubscriptionEndV1"
                                                        }
                                                        "channel.subscription.gift" => {
                                                            "ChannelSubscriptionGiftV1"
                                                        }
                                                        "channel.subscription.message" => {
                                                            "ChannelSubscriptionMessageV1"
                                                        }
                                                        "channel.cheer" => "ChannelCheerV1",
                                                        "channel.raid" => "ChannelRaidV1",
                                                        "channel.ban" => "ChannelBanV1",
                                                        "channel.unban" => "ChannelUnbanV1",
                                                        "channel.moderator.add" => {
                                                            "ChannelModeratorAddV1"
                                                        }
                                                        "channel.moderator.remove" => {
                                                            "ChannelModeratorRemoveV1"
                                                        }
                                                        _ => "",
                                                    };

                                                    // Extract the actual payload using the known struct name
                                                    let extracted_data = if !struct_name.is_empty()
                                                    {
                                                        extract_by_struct_name(
                                                            &event_json,
                                                            struct_name,
                                                        )
                                                    } else {
                                                        event_json.clone()
                                                    };

                                                    // Debug log to see the structure after extraction
                                                    debug!(
                                                        event_type = %event_type,
                                                        struct_name = %struct_name,
                                                        "Extracted payload: {}",
                                                        serde_json::to_string_pretty(&extracted_data).unwrap_or_default()
                                                    );

                                                    // For stream.offline, we don't have any data to send
                                                    let payload = if event_type == "stream.offline"
                                                    {
                                                        // Empty JSON object
                                                        json!({})
                                                    } else {
                                                        // For all other events, just use the extracted data directly
                                                        extracted_data
                                                    };

                                                    // Publish the event
                                                    let event = StreamEvent::new(
                                                        "twitch", event_type, payload,
                                                    );
                                                    if let Err(e) = event_bus.publish(event).await {
                                                        error!("Failed to publish event: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "Failed to convert event to JSON: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        EventsubWebsocketData::Revocation { payload, metadata } => {
                                            // Get the subscription ID from the event
                                            let subscription_id =
                                                match serde_json::to_value(&payload) {
                                                    Ok(Value::Object(obj)) => {
                                                        match obj.get("subscription") {
                                                            Some(Value::Object(sub)) => {
                                                                match sub.get("id") {
                                                                    Some(Value::String(id)) => {
                                                                        id.clone()
                                                                    }
                                                                    _ => String::from("unknown"),
                                                                }
                                                            }
                                                            _ => String::from("unknown"),
                                                        }
                                                    }
                                                    _ => String::from("unknown"),
                                                };

                                            warn!(
                                                "Subscription revoked: {} ({})",
                                                subscription_id, metadata.subscription_type
                                            );

                                            // Publish revocation event
                                            let payload = json!({
                                                "event": "subscription_revoked",
                                                "subscription_id": subscription_id,
                                                "subscription_type": metadata.subscription_type,
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                            });

                                            let event = StreamEvent::new(
                                                "twitch",
                                                "eventsub.revoked",
                                                payload,
                                            );

                                            if let Err(e) = event_bus.publish(event).await {
                                                error!("Failed to publish revocation event: {}", e);
                                            }
                                        }
                                        EventsubWebsocketData::Welcome { .. } => {
                                            // This should have been handled during connection
                                            debug!("Received welcome message after connection was established");
                                        }
                                        // Catch all for any new message types added in the future
                                        _ => {
                                            debug!("Received unhandled EventSub message type");
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse EventSub message: {}", e);
                                }
                            }
                        }
                        WsMessage::Ping(data) => {
                            // Respond to ping
                            debug!("Received ping, sending pong");
                            if let Err(e) = conn.ws_stream.send(WsMessage::Pong(data)).await {
                                error!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                        WsMessage::Pong(_) => {
                            // Received pong response
                            debug!("Received pong");
                        }
                        WsMessage::Close(frame) => {
                            info!("Received close frame: {:?}", frame);
                            break;
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    error!("WebSocket error: {}", e);

                    // Check if the error is related to authentication
                    let error_str = e.to_string();
                    if error_str.contains("401") || error_str.contains("Authentication") {
                        info!("Authentication error in WebSocket, attempting token refresh");

                        // Try to refresh the token
                        if let Some(refresher) = token_refresher.read().await.clone() {
                            match refresher().await {
                                Ok(new_token) => {
                                    info!("Token refreshed after WebSocket error");
                                    *token_storage.write().await = Some(new_token);
                                }
                                Err(refresh_err) => {
                                    error!(
                                        "Failed to refresh token after WebSocket error: {}",
                                        refresh_err
                                    );
                                }
                            }
                        }
                    }

                    break;
                }
                Ok(None) => {
                    info!("WebSocket closed");
                    break;
                }
                Err(_) => {
                    // Timeout, continue
                    continue;
                }
            }
        }

        // Cancel ping task
        ping_task.abort();
        info!("Stopped processing EventSub messages");
    }

    // The process_event_notification function has been integrated directly into the process_messages method
    // using the twitch_api library's event types and parsing
}

#[cfg(test)]
mod tests {
    pub use crate::adapters::tests::twitch_eventsub_test::*;
}
