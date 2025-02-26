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
    eventsub::{
        event::websocket::{EventsubWebsocketData, SessionData, WelcomePayload},
        Event,
    },
    types::UserId,
};
use twitch_oauth2::{ClientId, UserToken};

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
}

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
        })
    }

    /// Start the EventSub client
    pub async fn start(&self, token: &UserToken) -> Result<()> {
        info!("Starting Twitch EventSub client");

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Get broadcaster ID from token user ID
        let user_id = Self::get_user_id_from_token(&self.client_id, token).await?;
        *self.user_id.write().await = Some(user_id.to_string());

        // Clone Arc references for the background task
        let event_bus = self.event_bus.clone();
        let connection = self.connection.clone();
        let client_id = self.client_id.clone();
        let user_id_clone = self.user_id.clone();

        // Spawn the WebSocket connection task
        tokio::spawn(async move {
            // Track reconnect attempts for exponential backoff
            let mut reconnect_attempts = 0;

            // Connection loop
            loop {
                // Check for shutdown signal
                if let Ok(Some(_)) = timeout(Duration::from_millis(10), shutdown_rx.recv()).await {
                    info!("Shutdown signal received, stopping EventSub client");
                    break;
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

                        // Process messages until disconnected
                        Self::process_messages(&connection, &event_bus, &user_id_clone).await;
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

    /// Process EventSub WebSocket messages
    async fn process_messages(
        connection: &Arc<RwLock<Option<EventSubConnection>>>,
        event_bus: &Arc<EventBus>,
        user_id: &Arc<RwLock<Option<String>>>,
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
                                                    let broadcaster_id =
                                                        user_id.read().await.clone();

                                                    // Map the event type to our internal format
                                                    let event_type =
                                                        match metadata.subscription_type.to_str() {
                                                            "stream.online" => "stream.online",
                                                            "stream.offline" => "stream.offline",
                                                            "channel.update" => "channel.updated",
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
                                                        "channel.updated" => {
                                                            json!({
                                                                "channel": event_json,
                                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            })
                                                        }
                                                        _ => json!({}),
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
