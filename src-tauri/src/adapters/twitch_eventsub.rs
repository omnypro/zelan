use anyhow::{anyhow, Result};
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
use twitch_oauth2::{ClientId, UserToken, TwitchToken};

use crate::EventBus;
use crate::StreamEvent;

/// Default reconnect delay in seconds
const DEFAULT_RECONNECT_DELAY_SECS: u64 = 2;
/// Maximum reconnect delay in seconds (used for exponential backoff)
const MAX_RECONNECT_DELAY_SECS: u64 = 120;
/// WebSocket ping interval in seconds
const WEBSOCKET_PING_INTERVAL_SECS: u64 = 30;
/// WebSocket connection timeout in seconds
const WEBSOCKET_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Environment variable name for Twitch Client ID
const TWITCH_CLIENT_ID_ENV: &str = "TWITCH_CLIENT_ID";

/// Get the Twitch Client ID from environment
fn get_client_id() -> Result<ClientId> {
    match env::var(TWITCH_CLIENT_ID_ENV) {
        Ok(client_id) if !client_id.is_empty() => Ok(ClientId::new(client_id)),
        Ok(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is empty")),
        Err(_) => Err(anyhow!("TWITCH_CLIENT_ID environment variable is not set")),
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
pub type TokenRefresher = Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<UserToken>> + Send>> + Send + Sync>;

/// Twitch EventSub client
pub struct EventSubClient {
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Twitch Client ID
    client_id: ClientId,
    /// Active WebSocket connection
    connection: Arc<RwLock<Option<EventSubConnection>>>,
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

// We're no longer parsing chat messages and just passing through the raw data

impl EventSubClient {
    /// Create a new EventSub client
    pub fn new(event_bus: Arc<EventBus>) -> Result<Self> {
        // Get client ID from environment
        let client_id = get_client_id()?;

        Ok(Self {
            event_bus,
            client_id,
            connection: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            token: Arc::new(RwLock::new(None)),
            token_refresher: Arc::new(RwLock::new(None)),
            token_hash: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Set the token refresher callback
    pub fn set_token_refresher(&self, refresher: TokenRefresher) {
        let token_refresher = self.token_refresher.clone();
        tokio::spawn(async move {
            *token_refresher.write().await = Some(refresher);
        });
    }
    
    /// Update the token stored in the client
    pub async fn update_token(&self, token: UserToken) -> Result<()> {
        // Calculate token hash for change detection
        let token_hash = self.hash_token(&token);
        
        // Store token and its hash
        *self.token.write().await = Some(token);
        *self.token_hash.write().await = Some(token_hash);
        
        Ok(())
    }
    
    /// Get a fresh token (refresh if needed)
    pub async fn get_fresh_token(&self) -> Result<UserToken> {
        // First check if we need to refresh
        if let Err(e) = self.check_and_refresh_token_if_needed().await {
            error!("Error refreshing token during get_fresh_token: {}", e);
            // Continue anyway and try to use what we have
        }
        
        // Get the current token
        let token_guard = self.token.read().await;
        match &*token_guard {
            Some(token) => Ok(token.clone()),
            None => Err(anyhow!("No token available")),
        }
    }
    
    /// Check if token needs refresh and refresh it if needed
    pub async fn check_and_refresh_token_if_needed(&self) -> Result<()> {
        // Get current token
        let current_token = {
            let token_guard = self.token.read().await;
            match &*token_guard {
                Some(token) => token.clone(),
                None => return Err(anyhow!("No token available to refresh")),
            }
        };
        
        // Check if token is about to expire (within 10 minutes)
        let expires_in = current_token.expires_in();
        if expires_in.as_secs() < 600 {
            info!("Token expires in {} seconds, refreshing", expires_in.as_secs());
            
            // Get the refresher function
            let refresher = {
                let refresher_guard = self.token_refresher.read().await;
                match &*refresher_guard {
                    Some(refresher) => refresher.clone(),
                    None => return Err(anyhow!("No token refresher available")),
                }
            };
            
            // Call the refresher to get a new token
            match refresher().await {
                Ok(new_token) => {
                    // Calculate token hash
                    let token_hash = self.hash_token(&new_token);
                    let current_hash = {
                        let hash_guard = self.token_hash.read().await;
                        hash_guard.clone()
                    };
                    
                    // Update token if hash is different (token has changed)
                    if current_hash.is_none() || current_hash.unwrap() != token_hash {
                        info!("Token has changed, updating stored token");
                        self.update_token(new_token).await?;
                    } else {
                        info!("Token is unchanged after refresh");
                    }
                    
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to refresh token: {}", e);
                    Err(anyhow!("Failed to refresh token: {}", e))
                }
            }
        } else {
            // Token is still valid
            debug!("Token is still valid for {} seconds", expires_in.as_secs());
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
    pub async fn start(&self, token: &UserToken) -> Result<()> {
        info!("Starting Twitch EventSub client");

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Store the initial token
        self.update_token(token.clone()).await?;

        // Get broadcaster ID from token user ID
        let user_id = Self::get_user_id_from_token(&self.client_id, token).await?;
        *self.user_id.write().await = Some(user_id.to_string());

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
                if tokio::time::timeout(Duration::from_millis(1), token_check_interval.tick()).await.is_ok() {
                    // Token check interval reached
                    
                    // Get the current token
                    match token_storage.read().await.clone() {
                        Some(current_token) => {
                            // Check if token is expiring soon (within 10 minutes)
                            let expires_in = current_token.expires_in();
                            if expires_in.as_secs() < 600 {
                                info!("Token expiring in {} seconds, attempting refresh", expires_in.as_secs());
                                
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
                                            if connection.read().await.is_some() {
                                                info!("Reconnecting to use the new token");
                                                *connection.write().await = None;
                                                // Reset reconnect attempts since this is intentional
                                                reconnect_attempts = 0;
                                                continue;
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to refresh token: {}", e);
                                            // Continue anyway, will reconnect on 401
                                        }
                                    }
                                }
                            }
                        },
                        None => {
                            warn!("No token available for periodic check");
                        }
                    }
                }

                // Calculate reconnect delay with exponential backoff
                let reconnect_delay = if reconnect_attempts == 0 {
                    0 // First attempt, no delay
                } else {
                    let base_delay =
                        DEFAULT_RECONNECT_DELAY_SECS * 2u64.pow(reconnect_attempts as u32);
                    std::cmp::min(base_delay, MAX_RECONNECT_DELAY_SECS)
                };

                // Wait for reconnect delay if needed
                if reconnect_delay > 0 {
                    info!(
                        "Reconnecting to Twitch EventSub in {} seconds",
                        reconnect_delay
                    );
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
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
                        
                        info!("Creating EventSub subscriptions for broadcaster ID: {}", broadcaster_id);
                        
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
                            &session_id
                        ).await {
                            Ok(count) => {
                                info!("Successfully created {} EventSub subscriptions", count);
                            }
                            Err(e) => {
                                error!("Failed to create EventSub subscriptions: {}", e);
                                
                                // Check if the error is due to authentication (401)
                                if e.to_string().contains("401") || e.to_string().contains("Authentication") {
                                    info!("Authentication error during subscription creation, will attempt token refresh");
                                    
                                    // Try to refresh the token
                                    if let Some(refresher) = token_refresher.read().await.clone() {
                                        match refresher().await {
                                            Ok(new_token) => {
                                                info!("Token refreshed after 401 error");
                                                *token_storage.write().await = Some(new_token);
                                                // Force reconnection loop
                                                break;
                                            },
                                            Err(refresh_err) => {
                                                error!("Failed to refresh token after 401: {}", refresh_err);
                                            }
                                        }
                                    }
                                }
                                
                                // Continue anyway, we might handle some predefined subscriptions
                            }
                        }

                        // Process messages until disconnected
                        Self::process_messages(&connection, &event_bus, &user_id_clone, &token_storage, &token_refresher).await;
                    }
                    Err(e) => {
                        // Connection failed
                        error!("Failed to connect to Twitch EventSub: {}", e);
                        reconnect_attempts += 1;
                    }
                }

                // Clear connection before reconnecting
                *connection.write().await = None;
            }

            // Clear connection on shutdown
            *connection.write().await = None;
            info!("EventSub client stopped");
        });

        Ok(())
    }

    /// Stop the EventSub client
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Twitch EventSub client");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
        }

        // Clear connection
        *self.connection.write().await = None;

        Ok(())
    }

    /// Connect to Twitch EventSub WebSocket
    async fn connect_to_eventsub(
        connection: &Arc<RwLock<Option<EventSubConnection>>>,
    ) -> Result<String> {
        info!("Connecting to Twitch EventSub WebSocket");

        // EventSub WebSocket URL
        let ws_url = "wss://eventsub.wss.twitch.tv/ws";

        // Connect with timeout
        let ws_stream = match timeout(
            Duration::from_secs(WEBSOCKET_CONNECT_TIMEOUT_SECS),
            connect_async(ws_url),
        )
        .await
        {
            Ok(result) => match result {
                Ok((stream, _)) => stream,
                Err(e) => return Err(anyhow!("WebSocket connection failed: {}", e)),
            },
            Err(_) => return Err(anyhow!("WebSocket connection timed out")),
        };

        // Create connection with no session ID yet
        let mut conn = EventSubConnection {
            ws_stream,
            session_id: String::new(),
            last_keepalive: std::time::Instant::now(),
        };

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

                                // Update connection
                                *connection.write().await = Some(conn);

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
                    return Err(anyhow!("WebSocket error: {}", e));
                }
                Ok(None) => {
                    return Err(anyhow!("WebSocket closed unexpectedly"));
                }
                Err(_) => {
                    // Timeout on this iteration, continue
                    continue;
                }
            }
        }

        Err(anyhow!("Timed out waiting for welcome message"))
    }

    /// Get user ID from token
    async fn get_user_id_from_token(client_id: &ClientId, token: &UserToken) -> Result<UserId> {
        info!("Getting user ID from token");

        // Create client
        let client = reqwest::Client::new();

        // Validate token
        let response = client
            .get("https://api.twitch.tv/helix/users")
            .header("Client-ID", client_id.as_str())
            .header(
                "Authorization",
                format!("Bearer {}", token.access_token.secret()),
            )
            .send()
            .await?;

        // Check response
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!(
                "Failed to get user info: HTTP {} - {}",
                status,
                error_text
            ));
        }

        // Parse response
        let response_json: Value = response.json().await?;

        // Extract user ID
        if let Some(data) = response_json.get("data").and_then(|d| d.as_array()) {
            if let Some(user) = data.first() {
                if let Some(id) = user.get("id").and_then(|id| id.as_str()) {
                    return Ok(UserId::new(id.to_string()));
                }
            }
        }

        Err(anyhow!("Failed to extract user ID from response"))
    }
    
    /// Create EventSub subscriptions using create_eventsub_subscription() from the library
    async fn create_subscriptions(
        _client_id: &ClientId,
        token: &UserToken,
        broadcaster_id: &str,
        session_id: &str,
    ) -> Result<usize> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use twitch_api::{
            helix::HelixClient,
            eventsub::{
                channel::{
                    ChannelUpdateV2, ChannelChatMessageV1, ChannelFollowV2, 
                    ChannelSubscribeV1, ChannelSubscriptionEndV1, ChannelSubscriptionGiftV1,
                    ChannelSubscriptionMessageV1, ChannelCheerV1, ChannelRaidV1,
                    ChannelBanV1, ChannelUnbanV1, ChannelModeratorAddV1, ChannelModeratorRemoveV1,
                },
                stream::{StreamOfflineV1, StreamOnlineV1},
            },
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
            success_count: Arc<AtomicUsize>
        ) -> Result<()>
        where 
            C: twitch_api::eventsub::EventSubscription + std::fmt::Debug + Send
        {
            info!("Creating subscription for {}", name);
            match helix_client.create_eventsub_subscription(
                condition,
                transport.clone(),
                token,
            ).await {
                Ok(_) => {
                    info!("Successfully created subscription for {}", name);
                    success_count.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to create subscription for {}: {}", name, e);
                    Err(anyhow!("Failed to create subscription for {}: {}", name, e))
                }
            }
        }
        
        // Vector of (name, subscription creation function) pairs
        let subscriptions: Vec<(&str, Box<dyn FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send>)> = vec![
            // Stream events
            ("stream.online", Box::new(|| Box::pin(create_sub(
                "stream.online",
                &helix_client,
                StreamOnlineV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("stream.offline", Box::new(|| Box::pin(create_sub(
                "stream.offline",
                &helix_client,
                StreamOfflineV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            
            // Channel events
            ("channel.update", Box::new(|| Box::pin(create_sub(
                "channel.update",
                &helix_client,
                ChannelUpdateV2::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.follow", Box::new(|| Box::pin(create_sub(
                "channel.follow",
                &helix_client,
                ChannelFollowV2::new(broadcaster.clone(), broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.subscribe", Box::new(|| Box::pin(create_sub(
                "channel.subscribe",
                &helix_client,
                ChannelSubscribeV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.subscription.end", Box::new(|| Box::pin(create_sub(
                "channel.subscription.end",
                &helix_client,
                ChannelSubscriptionEndV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.subscription.gift", Box::new(|| Box::pin(create_sub(
                "channel.subscription.gift",
                &helix_client,
                ChannelSubscriptionGiftV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.subscription.message", Box::new(|| Box::pin(create_sub(
                "channel.subscription.message",
                &helix_client,
                ChannelSubscriptionMessageV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.cheer", Box::new(|| Box::pin(create_sub(
                "channel.cheer",
                &helix_client,
                ChannelCheerV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.raid", Box::new(|| Box::pin(create_sub(
                "channel.raid",
                &helix_client,
                ChannelRaidV1::to_broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.ban", Box::new(|| Box::pin(create_sub(
                "channel.ban",
                &helix_client,
                ChannelBanV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.unban", Box::new(|| Box::pin(create_sub(
                "channel.unban",
                &helix_client,
                ChannelUnbanV1::broadcaster_user_id(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.moderator.add", Box::new(|| Box::pin(create_sub(
                "channel.moderator.add",
                &helix_client,
                ChannelModeratorAddV1::new(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            ("channel.moderator.remove", Box::new(|| Box::pin(create_sub(
                "channel.moderator.remove",
                &helix_client,
                ChannelModeratorRemoveV1::new(broadcaster.clone()),
                &transport,
                token,
                success_count.clone()
            )))),
            
            // Chat message event
            ("channel.chat.message", Box::new(|| {
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
                    success_count.clone()
                ))
            })),
        ];
        
        // Create all subscriptions with delays between each
        for (_, create_sub_fn) in subscriptions {
            // Attempt to create the subscription
            let _ = create_sub_fn().await;
            
            // Add a small delay between subscriptions to prevent rate limiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Get the final subscription count
        let total_count = success_count.load(Ordering::SeqCst);
        info!("Created a total of {} EventSub subscriptions", total_count);
        
        // Return the count of successfully created subscriptions
        Ok(total_count)
    }

    /// Process EventSub WebSocket messages
    async fn process_messages(
        connection: &Arc<RwLock<Option<EventSubConnection>>>,
        event_bus: &Arc<EventBus>,
        user_id: &Arc<RwLock<Option<String>>>,
        token_storage: &Arc<RwLock<Option<UserToken>>>,
        token_refresher: &Arc<RwLock<Option<TokenRefresher>>>,
    ) {
        info!("Started processing EventSub messages");

        // Create ping timer
        let mut ping_interval =
            tokio::time::interval(Duration::from_secs(WEBSOCKET_PING_INTERVAL_SECS));

        // Set up a task to send periodic pings
        let connection_ping = connection.clone();
        let ping_task = tokio::spawn(async move {
            loop {
                ping_interval.tick().await;

                // Send ping if connection exists
                if let Some(conn) = &mut *connection_ping.write().await {
                    // Check if we've received a keepalive recently
                    let elapsed = conn.last_keepalive.elapsed();
                    debug!("Last keepalive: {:?} ago", elapsed);

                    // Send ping to keep connection alive
                    if let Err(e) = conn.ws_stream.send(WsMessage::Ping(vec![].into())).await {
                        error!("Failed to send ping: {}", e);
                        break;
                    }
                } else {
                    // Connection closed
                    break;
                }
            }

            debug!("Ping task stopped");
        });

        // Message processing loop
        loop {
            // Get a reference to the connection
            let mut conn_guard = connection.write().await;
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
                                                    let event_type =
                                                        match metadata.subscription_type.to_str() {
                                                            // Stream events
                                                            "stream.online" => "stream.online",
                                                            "stream.offline" => "stream.offline",
                                                            
                                                            // Channel events
                                                            "channel.update" => "channel.updated",
                                                            "channel.follow" => "channel.follow",
                                                            "channel.subscribe" => "channel.subscribe",
                                                            "channel.subscription.end" => "channel.subscription.end",
                                                            "channel.subscription.gift" => "channel.subscription.gift",
                                                            "channel.subscription.message" => "channel.subscription.message",
                                                            "channel.cheer" => "channel.cheer",
                                                            "channel.raid" => "channel.raid",
                                                            "channel.ban" => "channel.ban",
                                                            "channel.unban" => "channel.unban",
                                                            "channel.moderator.add" => "channel.moderator.add",
                                                            "channel.moderator.remove" => "channel.moderator.remove",
                                                            
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

                                                    // Create payload based on event type
                                                    let payload = match event_type {
                                                        // Stream events
                                                        "stream.online" => {
                                                            json!({
                                                                "stream": event_json,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        "stream.offline" => {
                                                            json!({
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        
                                                        // Channel update event
                                                        "channel.updated" => {
                                                            json!({
                                                                "channel": event_json,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        
                                                        // Chat message event
                                                        "chat.message" => {
                                                            // Pass through raw event data without parsing
                                                            json!({
                                                                "message": event_json,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        
                                                        // Channel-related events (follow, subscribe, etc.)
                                                        "channel.follow" | "channel.subscribe" | "channel.subscription.end" |
                                                        "channel.subscription.gift" | "channel.subscription.message" |
                                                        "channel.cheer" | "channel.raid" | "channel.ban" | "channel.unban" |
                                                        "channel.moderator.add" | "channel.moderator.remove" => {
                                                            json!({
                                                                "channel": event_json,
                                                                "event_type": event_type,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        
                                                        // For any other event types, use a generic format
                                                        _ => {
                                                            json!({
                                                                "data": event_json,
                                                                "event_type": event_type,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
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
                                },
                                Err(refresh_err) => {
                                    error!("Failed to refresh token after WebSocket error: {}", refresh_err);
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
    use super::*;
    use crate::EventBus;
    use std::sync::Arc;

    // Set up environment for tests
    fn mock_env_var() {
        std::env::set_var(TWITCH_CLIENT_ID_ENV, "test_client_id");
    }

    fn unmock_env_var() {
        std::env::remove_var(TWITCH_CLIENT_ID_ENV);
    }

    #[tokio::test]
    async fn test_eventsub_client_creation() {
        // Set up environment
        mock_env_var();

        // Create event bus with default capacity
        let event_bus = Arc::new(EventBus::new(100));

        // Create EventSub client
        let client = EventSubClient::new(event_bus);
        
        // Verify client was created successfully
        assert!(client.is_ok());
        
        // Clean up
        unmock_env_var();
    }
}
