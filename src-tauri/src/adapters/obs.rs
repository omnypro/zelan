// src/adapters/obs.rs
use crate::{
    adapters::{
        base::{AdapterConfig, BaseAdapter, ServiceAdapterHelper},
        common::{AdapterError, BackoffStrategy, RetryOptions, TraceHelper},
    },
    EventBus, ServiceAdapter, StreamEvent,
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use obws::{client::ConnectConfig, requests::EventSubscription, Client};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::async_runtime::{Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, instrument, warn, Instrument};

// Import the callback registry for OBS events
use crate::adapters::obs_callback::{ObsCallbackRegistry, ObsEvent};

// Default connection settings
pub(crate) const DEFAULT_HOST: &str = "localhost";
pub(crate) const DEFAULT_PORT: u16 = 4455;
pub(crate) const DEFAULT_PASSWORD: &str = "";
pub(crate) const RECONNECT_DELAY_MS: u64 = 5000;
// SHUTDOWN_CHANNEL_SIZE not needed anymore since we use BaseAdapter

/// Configuration for the OBS adapter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObsConfig {
    /// The host to connect to OBS WebSocket
    pub host: String,
    /// The port for OBS WebSocket
    pub port: u16,
    /// The password for OBS WebSocket (if required)
    pub password: String,
    /// Whether to auto-connect on startup
    pub auto_connect: bool,
    /// Whether to gather additional scene details on scene changes
    pub include_scene_details: bool,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            password: DEFAULT_PASSWORD.to_string(),
            auto_connect: true,
            include_scene_details: true,
        }
    }
}

impl AdapterConfig for ObsConfig {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "host": self.host,
            "port": self.port,
            "password": self.password,
            "auto_connect": self.auto_connect,
            "include_scene_details": self.include_scene_details,
        })
    }

    fn from_json(json: &serde_json::Value) -> Result<Self> {
        let mut config = ObsConfig::default();

        // Extract host if provided
        if let Some(host) = json.get("host").and_then(|v| v.as_str()) {
            config.host = host.to_string();
        }

        // Extract port if provided
        if let Some(port) = json.get("port").and_then(|v| v.as_u64()) {
            config.port = port as u16;
        }

        // Extract password if provided
        if let Some(password) = json.get("password").and_then(|v| v.as_str()) {
            config.password = password.to_string();
        }

        // Extract auto_connect if provided
        if let Some(auto_connect) = json.get("auto_connect").and_then(|v| v.as_bool()) {
            config.auto_connect = auto_connect;
        }

        // Extract include_scene_details if provided
        if let Some(include_scene_details) =
            json.get("include_scene_details").and_then(|v| v.as_bool())
        {
            config.include_scene_details = include_scene_details;
        }

        Ok(config)
    }

    fn adapter_type() -> &'static str {
        "obs"
    }

    fn validate(&self) -> Result<()> {
        // Ensure port is in a reasonable range
        if self.port == 0 {
            return Err(AdapterError::config("Port cannot be 0").into());
        }

        // Host cannot be empty
        if self.host.is_empty() {
            return Err(AdapterError::config("Host cannot be empty").into());
        }

        Ok(())
    }
}

/// OBS Studio adapter for connecting to OBS via its WebSocket API
pub struct ObsAdapter {
    /// Base adapter implementation
    base: BaseAdapter,
    /// OBS WebSocket client
    client: Arc<Mutex<Option<Arc<Client>>>>,
    /// Configuration specific to the OBS adapter
    config: Arc<RwLock<ObsConfig>>,
    /// Callback registry for OBS events
    callbacks: Arc<ObsCallbackRegistry>,
}

impl ObsAdapter {
    #[instrument(skip(event_bus), level = "debug")]
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        info!("Creating new OBS adapter");
        Self {
            base: BaseAdapter::new("obs", event_bus),
            client: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(ObsConfig::default())),
            callbacks: Arc::new(ObsCallbackRegistry::new()),
        }
    }

    /// Creates a new OBS adapter with the given configuration
    #[instrument(skip(event_bus), level = "debug")]
    pub fn with_config(event_bus: Arc<EventBus>, config: ObsConfig) -> Self {
        info!(
            host = %config.host,
            port = config.port,
            auto_connect = config.auto_connect,
            "Creating OBS adapter with custom config"
        );
        Self {
            base: BaseAdapter::new("obs", event_bus),
            client: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(config)),
            callbacks: Arc::new(ObsCallbackRegistry::new()),
        }
    }

    /// Register a callback for OBS events
    pub async fn register_obs_callback<F>(
        &self,
        callback: F,
    ) -> Result<crate::callback_system::CallbackId>
    where
        F: Fn(ObsEvent) -> Result<()> + Send + Sync + 'static,
    {
        // Record the callback registration in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "register_obs_callback",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        let id = self.callbacks.register(callback).await;

        // Record successful registration in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "register_obs_callback_success",
            Some(json!({
                "callback_id": id.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        info!(callback_id = %id, "Registered OBS event callback");
        Ok(id)
    }

    /// Get the current configuration (useful for testing)
    pub async fn get_config(&self) -> ObsConfig {
        self.config.read().await.clone()
    }

    /// Trigger an OBS event (useful for testing)
    pub async fn trigger_event(&self, event: ObsEvent) -> Result<()> {
        // Get event type before we move the event
        let event_type = event.event_type();

        // Record the event trigger in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "trigger_event",
            Some(json!({
                "event_type": event_type,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Set up retry options for triggering
        let trigger_retry_options = RetryOptions::new(
            2, // Max attempts
            BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(100),
            },
            true, // Add jitter
        );

        // Implement direct sequential retry logic
        let mut attempt = 0;
        let mut last_error: Option<AdapterError> = None;
        let mut success = false;
        let mut callback_count = 0;

        // Record the beginning of retry attempts in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "trigger_obs_event_start",
            Some(json!({
                "event_type": event.event_type(),
                "max_attempts": trigger_retry_options.max_attempts,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        while attempt < trigger_retry_options.max_attempts {
            attempt += 1;

            // Record the current attempt in trace
            TraceHelper::record_adapter_operation(
                "obs",
                "trigger_obs_event_attempt",
                Some(json!({
                    "attempt": attempt,
                    "event_type": event.event_type(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            // Attempt to trigger the event
            if attempt > 1 {
                debug!(attempt, "Retrying OBS event trigger");
            }

            match self.callbacks.trigger(event.clone()).await {
                Ok(count) => {
                    success = true;
                    callback_count = count;

                    // Record successful trigger in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "trigger_obs_event_success",
                        Some(json!({
                            "attempt": attempt,
                            "callbacks_triggered": count,
                            "event_type": event.event_type(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    debug!(callbacks = count, "Triggered OBS event successfully");
                    break;
                }
                Err(e) => {
                    // Convert to AdapterError
                    let error = AdapterError::from_anyhow_error(
                        "event",
                        format!("Failed to trigger OBS event: {}", e),
                        e,
                    );
                    last_error = Some(error.clone());

                    // Record failure in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "trigger_obs_event_attempt_failure",
                        Some(json!({
                            "attempt": attempt,
                            "error": error.to_string(),
                            "event_type": event.event_type(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // If not the last attempt, calculate delay and retry
                    if attempt < trigger_retry_options.max_attempts {
                        let delay = trigger_retry_options.get_delay(attempt);
                        warn!(
                            error = %error,
                            attempt = attempt,
                            next_delay_ms = %delay.as_millis(),
                            "OBS event trigger failed, retrying after delay"
                        );
                        sleep(delay).await;
                    } else {
                        error!(
                            error = %error,
                            attempts = attempt,
                            "Failed to trigger OBS event after maximum attempts"
                        );
                    }
                }
            }
        }

        // Determine result based on success flag
        let result = if success {
            Ok(callback_count)
        } else {
            Err(last_error.unwrap())
        };

        match result {
            Ok(count) => {
                debug!(callbacks = count, "Triggered OBS event successfully");

                // Record successful event trigger in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "trigger_event_success",
                    Some(json!({
                        "event_type": event_type,
                        "callbacks_triggered": count,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Failed to trigger OBS event");

                // Record event trigger failure in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "trigger_event_failure",
                    Some(json!({
                        "event_type": event_type,
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                Err(e.into())
            }
        }
    }

    /// Handles incoming OBS events and converts them to StreamEvents
    #[instrument(
        skip(client, event_bus, config, connected, shutdown_rx, callbacks),
        level = "debug"
    )]
    async fn handle_obs_events(
        client: Arc<Client>,
        event_bus: Arc<EventBus>,
        config: ObsConfig,
        connected: Arc<AtomicBool>,
        mut shutdown_rx: mpsc::Receiver<()>,
        callbacks: Arc<ObsCallbackRegistry>,
    ) -> Result<()> {
        // Record start of event handling in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "handle_events_start",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Set up OBS event listener
        let events = match client.events() {
            Ok(events) => events,
            Err(e) => {
                error!(error = %e, "Failed to create OBS event stream");

                // Record error in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "event_stream_creation_error",
                    Some(json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                return Err(AdapterError::connection_with_source(
                    format!("Failed to create OBS event stream: {}", e),
                    e,
                )
                .into());
            }
        };

        pin_mut!(events);
        info!("OBS event listener started");

        // Continue as long as connected is true and no shutdown signal is received
        loop {
            // Listen for the next event with a timeout to allow for checking connected state
            let maybe_event = tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal for OBS event handler");

                    // Record shutdown in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "event_handler_shutdown",
                        Some(json!({
                            "reason": "shutdown_signal",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    ).await;

                    break;
                }

                // Check connection status periodically
                _ = sleep(Duration::from_secs(5)) => {
                    if !connected.load(Ordering::SeqCst) {
                        info!("Connection flag set to false, stopping OBS event handler");

                        // Record disconnection in trace
                        TraceHelper::record_adapter_operation(
                            "obs",
                            "event_handler_shutdown",
                            Some(json!({
                                "reason": "disconnected",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        ).await;

                        break;
                    }
                    debug!("Connection check passed, OBS event handler still active");
                    continue;
                }

                // Try to get the next event
                ev = events.next() => ev,
            };

            if !connected.load(Ordering::SeqCst) {
                info!("Connection flag set to false, stopping OBS event handler");
                break;
            }

            // Handle next event from the stream
            match maybe_event {
                Some(event) => {
                    // Convert OBS event to our internal format
                    match event {
                        obws::events::Event::CurrentProgramSceneChanged { id } => {
                            let scene_name = id.name.clone();
                            debug!(scene = %scene_name, "OBS scene changed");

                            // Record scene change in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "scene_changed",
                                Some(json!({
                                    "scene_name": scene_name,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            let mut payload = json!({
                                "scene_name": scene_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            // If configured to include scene details, gather them
                            if config.include_scene_details {
                                debug!("Fetching additional scene details");

                                // Set up retry options for fetching scene details
                                let scene_details_retry = RetryOptions::new(
                                    2, // Max attempts
                                    BackoffStrategy::Exponential {
                                        base_delay: Duration::from_millis(10),
                                        max_delay: Duration::from_millis(100),
                                    },
                                    true, // Add jitter
                                );

                                // Implement direct sequential retry logic for scene details
                                let scene_name_for_attempts = id.name.clone();
                                let mut attempt = 0;
                                let mut last_error: Option<AdapterError> = None;
                                let mut success = false;
                                let mut scene_index_result: Option<usize> = None;

                                // Record the beginning of retry attempts in trace
                                TraceHelper::record_adapter_operation(
                                    "obs",
                                    "fetch_scene_details_start",
                                    Some(json!({
                                        "scene_name": scene_name_for_attempts,
                                        "max_attempts": scene_details_retry.max_attempts,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;

                                while attempt < scene_details_retry.max_attempts {
                                    attempt += 1;

                                    // Record the current attempt in trace
                                    TraceHelper::record_adapter_operation(
                                        "obs",
                                        "fetch_scene_details_attempt",
                                        Some(json!({
                                            "attempt": attempt,
                                            "scene_name": scene_name_for_attempts,
                                            "timestamp": chrono::Utc::now().to_rfc3339(),
                                        })),
                                    )
                                    .await;

                                    // Attempt to fetch scene details
                                    if attempt > 1 {
                                        debug!(attempt, "Retrying scene details fetch");
                                    }

                                    match client.scenes().list().await {
                                        Ok(scene_list) => {
                                            if let Some(scene) = scene_list
                                                .scenes
                                                .iter()
                                                .find(|s| s.id.name == scene_name_for_attempts)
                                            {
                                                success = true;
                                                scene_index_result = Some(scene.index);

                                                // Record successful fetch in trace
                                                TraceHelper::record_adapter_operation(
                                                    "obs",
                                                    "fetch_scene_details_success",
                                                    Some(json!({
                                                        "attempt": attempt,
                                                        "scene_name": scene_name_for_attempts,
                                                        "scene_index": scene.index,
                                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                                    })),
                                                ).await;

                                                break;
                                            } else {
                                                // Scene not found, but this is not an error
                                                success = true;
                                                scene_index_result = None;

                                                // Record scene not found in trace
                                                TraceHelper::record_adapter_operation(
                                                    "obs",
                                                    "fetch_scene_details_scene_not_found",
                                                    Some(json!({
                                                        "attempt": attempt,
                                                        "scene_name": scene_name_for_attempts,
                                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                                    })),
                                                ).await;

                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            // Convert to AdapterError
                                            let error = AdapterError::from_anyhow_error(
                                                "api",
                                                format!("Failed to fetch scene details: {}", e),
                                                anyhow::anyhow!("{}", e),
                                            );
                                            last_error = Some(error.clone());

                                            // Record failure in trace
                                            TraceHelper::record_adapter_operation(
                                                "obs",
                                                "fetch_scene_details_attempt_failure",
                                                Some(json!({
                                                    "attempt": attempt,
                                                    "error": error.to_string(),
                                                    "scene_name": scene_name_for_attempts,
                                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                                })),
                                            )
                                            .await;

                                            // If not the last attempt, calculate delay and retry
                                            if attempt < scene_details_retry.max_attempts {
                                                let delay = scene_details_retry.get_delay(attempt);
                                                warn!(
                                                    error = %error,
                                                    attempt = attempt,
                                                    next_delay_ms = %delay.as_millis(),
                                                    "Scene details fetch failed, retrying after delay"
                                                );
                                                sleep(delay).await;
                                            } else {
                                                error!(
                                                    error = %error,
                                                    attempts = attempt,
                                                    "Failed to fetch scene details after maximum attempts"
                                                );
                                            }
                                        }
                                    }
                                }

                                // Determine result based on success flag
                                let scene_details_result = if success {
                                    Ok(scene_index_result)
                                } else {
                                    Err(last_error.unwrap())
                                };

                                // Process the result
                                match scene_details_result {
                                    Ok(Some(scene_index)) => {
                                        payload["scene_index"] = json!(scene_index);
                                        debug!(scene_index, "Added scene details");
                                    }
                                    Ok(None) => {
                                        debug!("Scene not found in scene list");
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "Failed to fetch scene details after retries");
                                    }
                                }
                            }

                            // Set up retry options for publishing events
                            let publish_retry = RetryOptions::new(
                                2, // Max attempts
                                BackoffStrategy::Exponential {
                                    base_delay: Duration::from_millis(10),
                                    max_delay: Duration::from_millis(100),
                                },
                                true, // Add jitter
                            );

                            // Implement direct sequential retry logic for publishing
                            let payload_for_publish = payload.clone();
                            let mut attempt = 0;
                            let mut last_error: Option<AdapterError> = None;
                            let mut success = false;
                            let mut receivers_count = 0;

                            // Record the beginning of publish attempts in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_scene_change_start",
                                Some(json!({
                                    "scene_name": scene_name,
                                    "max_attempts": publish_retry.max_attempts,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            while attempt < publish_retry.max_attempts {
                                attempt += 1;

                                // Record the current attempt in trace
                                TraceHelper::record_adapter_operation(
                                    "obs",
                                    "publish_scene_change_attempt",
                                    Some(json!({
                                        "attempt": attempt,
                                        "scene_name": scene_name,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;

                                // Attempt to publish the event
                                if attempt > 1 {
                                    debug!(attempt, "Retrying scene change event publication");
                                }

                                let stream_event = StreamEvent::new(
                                    "obs",
                                    "scene.changed",
                                    payload_for_publish.clone(),
                                );
                                match event_bus.publish(stream_event).await {
                                    Ok(receivers) => {
                                        success = true;
                                        receivers_count = receivers;

                                        // Record successful publish in trace
                                        TraceHelper::record_adapter_operation(
                                            "obs",
                                            "publish_scene_change_success",
                                            Some(json!({
                                                "attempt": attempt,
                                                "receivers": receivers,
                                                "scene_name": scene_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                            })),
                                        )
                                        .await;

                                        break;
                                    }
                                    Err(e) => {
                                        // Convert to AdapterError
                                        let error = AdapterError::from_anyhow_error(
                                            "event",
                                            format!("Failed to publish scene change event: {}", e),
                                            anyhow::anyhow!("{}", e),
                                        );
                                        last_error = Some(error.clone());

                                        // Record failure in trace
                                        TraceHelper::record_adapter_operation(
                                            "obs",
                                            "publish_scene_change_attempt_failure",
                                            Some(json!({
                                                "attempt": attempt,
                                                "error": error.to_string(),
                                                "scene_name": scene_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                            })),
                                        )
                                        .await;

                                        // If not the last attempt, calculate delay and retry
                                        if attempt < publish_retry.max_attempts {
                                            let delay = publish_retry.get_delay(attempt);
                                            warn!(
                                                error = %error,
                                                attempt = attempt,
                                                next_delay_ms = %delay.as_millis(),
                                                "Scene change publication failed, retrying after delay"
                                            );
                                            sleep(delay).await;
                                        } else {
                                            error!(
                                                error = %error,
                                                attempts = attempt,
                                                "Failed to publish scene change event after maximum attempts"
                                            );
                                        }
                                    }
                                }
                            }

                            // Determine result based on success flag
                            let publish_result = if success {
                                Ok(receivers_count)
                            } else {
                                Err(last_error.unwrap())
                            };

                            // Process publish result
                            match publish_result {
                                Ok(receivers) => {
                                    debug!(scene = %scene_name, receivers, "Published scene change event");
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to publish OBS scene change event");
                                }
                            }

                            // Create callback event
                            let callback_event = ObsEvent::SceneChanged {
                                scene_name: scene_name.clone(),
                                data: payload,
                            };

                            // Implement direct sequential retry logic for triggering callbacks
                            let mut attempt = 0;
                            let mut last_error: Option<AdapterError> = None;
                            let mut success = false;
                            let mut callbacks_triggered = 0;

                            // Record the beginning of trigger attempts in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "trigger_scene_change_callbacks_start",
                                Some(json!({
                                    "scene_name": scene_name,
                                    "max_attempts": publish_retry.max_attempts,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            while attempt < publish_retry.max_attempts {
                                attempt += 1;

                                // Record the current attempt in trace
                                TraceHelper::record_adapter_operation(
                                    "obs",
                                    "trigger_scene_change_callbacks_attempt",
                                    Some(json!({
                                        "attempt": attempt,
                                        "scene_name": scene_name,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    })),
                                )
                                .await;

                                // Attempt to trigger callbacks
                                if attempt > 1 {
                                    debug!(attempt, "Retrying scene change callback trigger");
                                }

                                match callbacks.trigger(callback_event.clone()).await {
                                    Ok(count) => {
                                        success = true;
                                        callbacks_triggered = count;

                                        // Record successful trigger in trace
                                        TraceHelper::record_adapter_operation(
                                            "obs",
                                            "trigger_scene_change_callbacks_success",
                                            Some(json!({
                                                "attempt": attempt,
                                                "callbacks_triggered": count,
                                                "scene_name": scene_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                            })),
                                        )
                                        .await;

                                        break;
                                    }
                                    Err(e) => {
                                        // Convert to AdapterError
                                        let error = AdapterError::from_anyhow_error(
                                            "event",
                                            format!(
                                                "Failed to trigger scene change callbacks: {}",
                                                e
                                            ),
                                            e,
                                        );
                                        last_error = Some(error.clone());

                                        // Record failure in trace
                                        TraceHelper::record_adapter_operation(
                                            "obs",
                                            "trigger_scene_change_callbacks_attempt_failure",
                                            Some(json!({
                                                "attempt": attempt,
                                                "error": error.to_string(),
                                                "scene_name": scene_name,
                                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                            })),
                                        )
                                        .await;

                                        // If not the last attempt, calculate delay and retry
                                        if attempt < publish_retry.max_attempts {
                                            let delay = publish_retry.get_delay(attempt);
                                            warn!(
                                                error = %error,
                                                attempt = attempt,
                                                next_delay_ms = %delay.as_millis(),
                                                "Scene change callbacks trigger failed, retrying after delay"
                                            );
                                            sleep(delay).await;
                                        } else {
                                            error!(
                                                error = %error,
                                                attempts = attempt,
                                                "Failed to trigger scene change callbacks after maximum attempts"
                                            );
                                        }
                                    }
                                }
                            }

                            // Determine result based on success flag
                            let trigger_result = if success {
                                Ok(callbacks_triggered)
                            } else {
                                Err(last_error.unwrap())
                            };

                            // Process trigger result
                            match trigger_result {
                                Ok(count) => {
                                    debug!(scene = %scene_name, callbacks = count, "Triggered scene change callbacks");
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to trigger OBS scene change callbacks");
                                }
                            }
                        }
                        obws::events::Event::SceneItemEnableStateChanged {
                            scene,
                            item_id,
                            enabled,
                            ..
                        } => {
                            debug!(
                                scene = ?scene,
                                item_id = ?item_id,
                                enabled = enabled,
                                "Scene item visibility changed"
                            );

                            let payload = json!({
                                "scene_name": scene,
                                "scene_item_id": item_id,
                                "enabled": enabled,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "scene_item.visibility_changed", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS scene item event");
                            }
                        }
                        obws::events::Event::StreamStateChanged { active, .. } => {
                            let event_type = if active {
                                "stream.started"
                            } else {
                                "stream.stopped"
                            };

                            info!(
                                active = active,
                                event_type = %event_type,
                                "OBS stream state changed"
                            );

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload.clone());
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS stream state event");
                            }

                            // Also trigger callbacks
                            let callback_event = ObsEvent::StreamStateChanged {
                                active,
                                data: payload,
                            };

                            if let Err(e) = callbacks.trigger(callback_event).await {
                                error!(error = %e, "Failed to trigger OBS stream state callbacks");
                            } else {
                                debug!(active = active, "Triggered stream state callbacks");
                            }
                        }
                        obws::events::Event::RecordStateChanged { active, .. } => {
                            let event_type = if active {
                                "recording.started"
                            } else {
                                "recording.stopped"
                            };

                            info!(
                                active = active,
                                event_type = %event_type,
                                "OBS recording state changed"
                            );

                            let payload = json!({
                                "active": active,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", event_type, payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS recording state event");
                            }
                        }
                        // Provide connection events
                        obws::events::Event::ExitStarted => {
                            warn!("OBS is shutting down");

                            let payload = json!({
                                "message": "OBS is shutting down",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event =
                                StreamEvent::new("obs", "connection.closing", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish OBS exit event");
                            }

                            // OBS is shutting down, so we should stop
                            connected.store(false, Ordering::SeqCst);
                            break;
                        }
                        // Catch all other events and log them with generic handling
                        _ => {
                            // We can add specific handling for more event types as needed
                            let event_name = format!("{:?}", event);
                            debug!(event = %event_name, "Received other OBS event");

                            // Create a simple payload with event details
                            let payload = json!({
                                "event_type": event_name,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });

                            let stream_event = StreamEvent::new("obs", "event.generic", payload);
                            if let Err(e) = event_bus.publish(stream_event).await {
                                error!(error = %e, "Failed to publish generic OBS event");
                            }
                        }
                    }
                }
                // In obws 0.14 the events stream returns Event directly, not Result<Event, Error>
                // This branch is kept for future compatibility or in case errors are added
                None => {
                    // End of events stream usually means connection closed
                    info!("OBS events stream ended");

                    // If we're still supposed to be connected, this is unexpected
                    if connected.load(Ordering::SeqCst) {
                        warn!("OBS WebSocket connection closed unexpectedly while still enabled");

                        // Send a disconnection event
                        let payload = json!({
                            "message": "OBS WebSocket connection closed unexpectedly",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });

                        let stream_event = StreamEvent::new("obs", "connection.closed", payload);
                        if let Err(e) = event_bus.publish(stream_event).await {
                            error!(error = %e, "Failed to publish OBS disconnection event");
                        }

                        // We're no longer connected
                        connected.store(false, Ordering::SeqCst);
                    } else {
                        debug!("OBS events stream ended normally (adapter was disabled)");
                    }

                    break;
                }
            }
        }

        info!("OBS event handler stopped");
        Ok(())
    }

    /// Get current scenes and publish an initial state
    #[instrument(skip(client, event_bus), level = "debug")]
    async fn publish_initial_state(client: &Arc<Client>, event_bus: &Arc<EventBus>) -> Result<()> {
        info!("Publishing initial OBS state");

        // Get version information
        match client.general().version().await {
            Ok(version) => {
                info!(
                    obs_version = %version.obs_version,
                    websocket_version = %version.obs_web_socket_version,
                    platform = %version.platform,
                    "OBS system info"
                );

                let payload = json!({
                    "obs_version": version.obs_version,
                    "websocket_version": version.obs_web_socket_version,
                    "platform": version.platform,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "system.info", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish OBS version info");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch OBS version info");
            }
        }

        // Get current scene
        match client.scenes().current_program_scene().await {
            Ok(current_scene) => {
                info!(scene = ?current_scene, "Current scene");

                let payload = json!({
                    "scene_name": current_scene,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "scene.current", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish current scene");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch current scene");
            }
        }

        // Get all scenes
        match client.scenes().list().await {
            Ok(scenes) => {
                debug!(scene_count = scenes.scenes.len(), "Retrieved scene list");

                let payload = json!({
                    "scenes": scenes.scenes,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let stream_event = StreamEvent::new("obs", "scenes.list", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish scenes list");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch scenes list");
            }
        }

        // Get stream status
        match client.streaming().status().await {
            Ok(status) => {
                info!(active = status.active, "Stream status");

                let payload = json!({
                    "active": status.active,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "stream.status", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish stream status");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch stream status");
            }
        }

        // Get recording status
        match client.recording().status().await {
            Ok(status) => {
                info!(active = status.active, "Recording status");

                let payload = json!({
                    "active": status.active,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "initial": true,
                });

                let stream_event = StreamEvent::new("obs", "recording.status", payload);
                if let Err(e) = event_bus.publish(stream_event).await {
                    error!(error = %e, "Failed to publish recording status");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch recording status");
            }
        }

        info!("Initial OBS state published");
        Ok(())
    }
}

#[async_trait]
impl ServiceAdapter for ObsAdapter {
    #[instrument(skip(self), fields(adapter = "obs"), level = "debug")]
    async fn connect(&self) -> Result<()> {
        // Record the connect operation in trace
        TraceHelper::record_adapter_operation(
            "obs",
            "connect",
            Some(json!({
                "already_connected": self.base.is_connected(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        // Check if already connected
        if self.base.is_connected() {
            info!("OBS adapter is already connected");
            return Ok(());
        }

        // Get connection configuration
        let config = self.config.read().await.clone();

        // Connect to OBS WebSocket server
        info!(
            host = %config.host,
            port = config.port,
            "Connecting to OBS WebSocket"
        );

        // Set up retry options for OBS connection
        let connect_retry_options = RetryOptions::new(
            3, // Max attempts
            BackoffStrategy::Exponential {
                base_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(5),
            },
            true, // Add jitter
        );

        // Log the connection parameters
        debug!(
            host = %config.host,
            port = config.port,
            password_provided = !config.password.is_empty(),
            "Preparing OBS connection parameters"
        );

        // Implement direct sequential retry logic to avoid type recursion issues
        // This matches the pattern we implemented for the Twitch adapter
        let mut attempt = 0;
        let mut client: Option<Arc<Client>> = None;
        let mut last_error: Option<AdapterError> = None;

        // Record that we're starting retry attempts
        TraceHelper::record_adapter_operation(
            "obs",
            "connect_retry_start",
            Some(json!({
                "host": config.host,
                "port": config.port,
                "max_attempts": connect_retry_options.max_attempts,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        )
        .await;

        while attempt < connect_retry_options.max_attempts {
            attempt += 1;
            debug!(attempt, "Attempting to connect to OBS");

            // Recreate the connection options for each attempt
            let connect_opts = ConnectConfig {
                host: config.host.clone(),
                port: config.port,
                password: if config.password.is_empty() {
                    None
                } else {
                    Some(config.password.clone())
                },
                event_subscriptions: Some(EventSubscription::ALL),
                broadcast_capacity: 128,
                connect_timeout: Duration::from_secs(10),
                dangerous: None,
            };

            // Record the attempt in trace
            TraceHelper::record_adapter_operation(
                "obs",
                "connect_attempt",
                Some(json!({
                    "attempt": attempt,
                    "host": config.host,
                    "port": config.port,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })),
            )
            .await;

            // Attempt to connect
            match Client::connect_with_config(connect_opts).await {
                Ok(new_client) => {
                    // Connection successful
                    client = Some(Arc::new(new_client));

                    // Record success in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "connect_attempt_success",
                        Some(json!({
                            "attempt": attempt,
                            "host": config.host,
                            "port": config.port,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    break;
                }
                Err(e) => {
                    // Connection failed
                    let error = AdapterError::connection_with_source(
                        format!("Failed to connect to OBS WebSocket: {}", e),
                        e,
                    );
                    last_error = Some(error.clone());

                    // Record failure in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "connect_attempt_failure",
                        Some(json!({
                            "attempt": attempt,
                            "host": config.host,
                            "port": config.port,
                            "error": error.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // If this wasn't the last attempt, calculate delay and retry
                    if attempt < connect_retry_options.max_attempts {
                        let delay = connect_retry_options.get_delay(attempt);
                        warn!(
                            error = %error,
                            attempt = attempt,
                            next_delay_ms = %delay.as_millis(),
                            "OBS connection failed, retrying after delay"
                        );
                        sleep(delay).await;
                    } else {
                        error!(
                            error = %error,
                            attempts = attempt,
                            "Failed to connect to OBS after maximum attempts"
                        );
                    }
                }
            }
        }

        // Process the final result
        let connect_result = match client {
            Some(client) => Ok(client),
            None => Err(last_error.unwrap()),
        };

        // Process connection result
        match connect_result {
            Ok(client) => {
                // Store the client
                *self.client.lock().await = Some(Arc::clone(&client));

                // Mark as connected using BaseAdapter
                self.base.set_connected(true);

                info!("Connected to OBS WebSocket server");

                // Record successful connection in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "connect_success",
                    Some(json!({
                        "host": config.host,
                        "port": config.port,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                // Publish initial state using direct sequential retry logic
                let initial_state_retry_options = RetryOptions::default();
                let mut attempt = 0;
                let mut initial_state_error: Option<AdapterError> = None;
                let mut initial_state_success = false;

                // Record that we're starting initial state publication
                TraceHelper::record_adapter_operation(
                    "obs",
                    "publish_initial_state_start",
                    Some(json!({
                        "max_attempts": initial_state_retry_options.max_attempts,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                while attempt < initial_state_retry_options.max_attempts {
                    attempt += 1;
                    debug!(attempt, "Attempting to publish initial OBS state");

                    // Record the attempt in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "publish_initial_state_attempt",
                        Some(json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Attempt to publish initial state
                    match Self::publish_initial_state(&client, &self.base.event_bus()).await {
                        Ok(_) => {
                            // Publication successful
                            initial_state_success = true;

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_initial_state_success",
                                Some(json!({
                                    "attempt": attempt,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            debug!("Published initial OBS state successfully");
                            break;
                        }
                        Err(e) => {
                            // Publication failed
                            let error = AdapterError::from_anyhow_error(
                                "internal",
                                format!("Failed to publish initial OBS state: {}", e),
                                e,
                            );
                            initial_state_error = Some(error.clone());

                            // Record failure in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_initial_state_attempt_failure",
                                Some(json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            // If this wasn't the last attempt, calculate delay and retry
                            if attempt < initial_state_retry_options.max_attempts {
                                let delay = initial_state_retry_options.get_delay(attempt);
                                warn!(
                                    error = %error,
                                    attempt = attempt,
                                    next_delay_ms = %delay.as_millis(),
                                    "Initial state publication failed, retrying after delay"
                                );
                                sleep(delay).await;
                            } else {
                                error!(
                                    error = %error,
                                    attempts = attempt,
                                    "Failed to publish initial OBS state after maximum attempts"
                                );
                            }
                        }
                    }
                }

                // Process final result of initial state publication
                if !initial_state_success && initial_state_error.is_some() {
                    error!(
                        error = %initial_state_error.unwrap(),
                        "Failed to publish initial OBS state after retries"
                    );
                } else if initial_state_success {
                    debug!("Published initial OBS state");
                }

                // Create channel for shutdown signaling using BaseAdapter
                let (_, shutdown_rx) = self.base.create_shutdown_channel().await;

                // Start event handler
                // Create a connected flag for the event handler
                let connected_flag = Arc::new(AtomicBool::new(true));

                // Clone what we need from self for the async block
                let event_bus = self.base.event_bus();
                let callbacks = self.callbacks.clone();
                let config_clone = config.clone();

                let handle = tauri::async_runtime::spawn(
                    async move {
                        if let Err(e) = Self::handle_obs_events(
                            client,
                            event_bus,
                            config_clone,
                            connected_flag,
                            shutdown_rx,
                            callbacks,
                        )
                        .await
                        {
                            error!(error = %e, "OBS event handler error");
                        }
                    }
                    .in_current_span(),
                );

                // Store the event handler using BaseAdapter
                self.base.set_event_handler(handle).await;

                // Create a copy of the connection configuration data we need
                let host = config.host.clone();
                let port = config.port;

                // Set up connection established event with payload
                let payload = json!({
                    "host": host,
                    "port": port,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                // Use direct sequential retry logic for publishing connection event
                let publish_retry_options = RetryOptions::default();
                let mut attempt = 0;
                let mut publish_error: Option<AdapterError> = None;
                let mut publish_success = false;
                let mut receivers_count = 0;

                // Record that we're starting event publication
                TraceHelper::record_adapter_operation(
                    "obs",
                    "publish_connection_event_start",
                    Some(json!({
                        "max_attempts": publish_retry_options.max_attempts,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                while attempt < publish_retry_options.max_attempts {
                    attempt += 1;
                    debug!(attempt, "Attempting to publish connection event");

                    // Record the attempt in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "publish_connection_event_attempt",
                        Some(json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Create the event and attempt to publish
                    let stream_event =
                        StreamEvent::new("obs", "connection.established", payload.clone());
                    match self.base.event_bus().publish(stream_event).await {
                        Ok(receivers) => {
                            // Publication successful
                            publish_success = true;
                            receivers_count = receivers;

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_connection_event_success",
                                Some(json!({
                                    "attempt": attempt,
                                    "receivers": receivers,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            debug!(receivers, "Published OBS connection event successfully");
                            break;
                        }
                        Err(e) => {
                            // Publication failed
                            let error = AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to publish connection event: {}", e),
                                anyhow::anyhow!("{}", e),
                            );
                            publish_error = Some(error.clone());

                            // Record failure in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_connection_event_attempt_failure",
                                Some(json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            // If this wasn't the last attempt, calculate delay and retry
                            if attempt < publish_retry_options.max_attempts {
                                let delay = publish_retry_options.get_delay(attempt);
                                warn!(
                                    error = %error,
                                    attempt = attempt,
                                    next_delay_ms = %delay.as_millis(),
                                    "Connection event publication failed, retrying after delay"
                                );
                                sleep(delay).await;
                            } else {
                                error!(
                                    error = %error,
                                    attempts = attempt,
                                    "Failed to publish connection event after maximum attempts"
                                );
                            }
                        }
                    }
                }

                // Process final result of event publication
                if !publish_success && publish_error.is_some() {
                    error!(
                        error = %publish_error.unwrap(),
                        "Failed to publish OBS connection event after retries"
                    );
                } else if publish_success {
                    debug!(
                        receivers = receivers_count,
                        "Published OBS connection event"
                    );
                }

                // Now trigger connection callbacks with direct sequential retry logic
                let callback_event = ObsEvent::ConnectionChanged {
                    connected: true,
                    data: payload,
                };

                let trigger_retry_options = RetryOptions::default();
                let mut attempt = 0;
                let mut trigger_error: Option<AdapterError> = None;
                let mut trigger_success = false;
                let mut callbacks_triggered = 0;

                // Record that we're starting callback trigger
                TraceHelper::record_adapter_operation(
                    "obs",
                    "trigger_connection_callbacks_start",
                    Some(json!({
                        "max_attempts": trigger_retry_options.max_attempts,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                while attempt < trigger_retry_options.max_attempts {
                    attempt += 1;
                    debug!(attempt, "Attempting to trigger connection callbacks");

                    // Record the attempt in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "trigger_connection_callbacks_attempt",
                        Some(json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Attempt to trigger callbacks
                    match self.callbacks.trigger(callback_event.clone()).await {
                        Ok(count) => {
                            // Trigger successful
                            trigger_success = true;
                            callbacks_triggered = count;

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "trigger_connection_callbacks_success",
                                Some(json!({
                                    "attempt": attempt,
                                    "callbacks_triggered": count,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            debug!(
                                callbacks = count,
                                "Triggered OBS connection callbacks successfully"
                            );
                            break;
                        }
                        Err(e) => {
                            // Trigger failed
                            let error = AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to trigger connection callbacks: {}", e),
                                e,
                            );
                            trigger_error = Some(error.clone());

                            // Record failure in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "trigger_connection_callbacks_attempt_failure",
                                Some(json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            // If this wasn't the last attempt, calculate delay and retry
                            if attempt < trigger_retry_options.max_attempts {
                                let delay = trigger_retry_options.get_delay(attempt);
                                warn!(
                                    error = %error,
                                    attempt = attempt,
                                    next_delay_ms = %delay.as_millis(),
                                    "Connection callbacks trigger failed, retrying after delay"
                                );
                                sleep(delay).await;
                            } else {
                                error!(
                                    error = %error,
                                    attempts = attempt,
                                    "Failed to trigger connection callbacks after maximum attempts"
                                );
                            }
                        }
                    }
                }

                // Process final result of trigger
                if !trigger_success && trigger_error.is_some() {
                    error!(
                        error = %trigger_error.unwrap(),
                        "Failed to trigger OBS connection callbacks after retries"
                    );
                } else if trigger_success {
                    debug!(
                        callbacks = callbacks_triggered,
                        "Triggered OBS connection callbacks"
                    );
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    error = %e,
                    host = %config.host,
                    port = config.port,
                    "Failed to connect to OBS WebSocket after retries"
                );

                // Record connection failure in trace
                TraceHelper::record_adapter_operation(
                    "obs",
                    "connect_failure",
                    Some(json!({
                        "host": config.host,
                        "port": config.port,
                        "error": e.to_string(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                // Use direct sequential retry logic for publishing failure event
                let payload = json!({
                    "host": config.host,
                    "port": config.port,
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });

                let failure_retry_options = RetryOptions::default();
                let mut attempt = 0;
                let mut failure_error: Option<AdapterError> = None;
                let mut failure_success = false;

                // Record that we're starting failure event publication
                TraceHelper::record_adapter_operation(
                    "obs",
                    "publish_connection_failed_start",
                    Some(json!({
                        "max_attempts": failure_retry_options.max_attempts,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    })),
                )
                .await;

                while attempt < failure_retry_options.max_attempts {
                    attempt += 1;
                    debug!(attempt, "Attempting to publish connection failure event");

                    // Record the attempt in trace
                    TraceHelper::record_adapter_operation(
                        "obs",
                        "publish_connection_failed_attempt",
                        Some(json!({
                            "attempt": attempt,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        })),
                    )
                    .await;

                    // Create the event and attempt to publish
                    let stream_event =
                        StreamEvent::new("obs", "connection.failed", payload.clone());
                    match self.base.event_bus().publish(stream_event).await {
                        Ok(_) => {
                            // Publication successful
                            failure_success = true;

                            // Record success in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_connection_failed_success",
                                Some(json!({
                                    "attempt": attempt,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            debug!("Published OBS connection failure event successfully");
                            break;
                        }
                        Err(pub_err) => {
                            // Publication failed
                            let error = AdapterError::from_anyhow_error(
                                "event",
                                format!("Failed to publish connection failure event: {}", pub_err),
                                anyhow::anyhow!("{}", pub_err),
                            );
                            failure_error = Some(error.clone());

                            // Record failure in trace
                            TraceHelper::record_adapter_operation(
                                "obs",
                                "publish_connection_failed_attempt_failure",
                                Some(json!({
                                    "attempt": attempt,
                                    "error": error.to_string(),
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                })),
                            )
                            .await;

                            // If this wasn't the last attempt, calculate delay and retry
                            if attempt < failure_retry_options.max_attempts {
                                let delay = failure_retry_options.get_delay(attempt);
                                warn!(
                                    error = %error,
                                    attempt = attempt,
                                    next_delay_ms = %delay.as_millis(),
                                    "Connection failure event publication failed, retrying after delay"
                                );
                                sleep(delay).await;
                            } else {
                                error!(
                                    error = %error,
                                    attempts = attempt,
                                    "Failed to publish connection failure event after maximum attempts"
                                );
                            }
                        }
                    }
                }

                // Process final result of failure event publication
                if !failure_success && failure_error.is_some() {
                    error!(
                        error = %failure_error.unwrap(),
                        "Failed to publish OBS connection failure event after retries"
                    );
                } else if failure_success {
                    debug!("Published OBS connection failure event");
                }

                // Also try to trigger callbacks for the failure
                let callback_event = ObsEvent::ConnectionChanged {
                    connected: false,
                    data: payload,
                };

                // We can attempt to trigger callbacks but it's not critical if it fails
                if let Err(trigger_err) = self.callbacks.trigger(callback_event).await {
                    warn!(
                        error = %trigger_err,
                        "Failed to trigger connection failure callbacks"
                    );
                }

                Err(e.into())
            }
        }
    }

    #[instrument(skip(self), fields(adapter = "obs"), level = "debug")]
    async fn disconnect(&self) -> Result<()> {
        // Use the default implementation from ServiceAdapterHelper
        // The cleanup logic for clearing the client reference is implemented
        // in the clean_up_on_disconnect hook method
        self.disconnect_default().await
    }

    fn is_connected(&self) -> bool {
        self.is_connected_default()
    }

    fn get_name(&self) -> &str {
        self.get_name_default()
    }

    #[instrument(skip(self, config), fields(adapter = "obs"), level = "debug")]
    async fn configure(&self, config: serde_json::Value) -> Result<()> {
        info!("Configuring OBS adapter");
        let current_config = self.config.read().await.clone();
        let mut new_config = current_config.clone();

        // Track if any connection-related settings changed
        let mut connection_settings_changed = false;

        // Update host if provided
        if let Some(host) = config.get("host").and_then(|v| v.as_str()) {
            if host != current_config.host {
                info!(old_host = %current_config.host, new_host = %host, "Updating OBS host");
                new_config.host = host.to_string();
                connection_settings_changed = true;
            }
        }

        // Update port if provided
        if let Some(port) = config.get("port").and_then(|v| v.as_u64()) {
            let port = port as u16;
            if port != current_config.port {
                info!(
                    old_port = current_config.port,
                    new_port = port,
                    "Updating OBS port"
                );
                new_config.port = port;
                connection_settings_changed = true;
            }
        }

        // Update password if provided
        if let Some(password) = config.get("password").and_then(|v| v.as_str()) {
            if password != current_config.password {
                debug!("Updating OBS password");
                new_config.password = password.to_string();
                connection_settings_changed = true;
            }
        }

        // Update auto_connect if provided
        if let Some(auto_connect) = config.get("auto_connect").and_then(|v| v.as_bool()) {
            if auto_connect != current_config.auto_connect {
                info!(
                    old_auto_connect = current_config.auto_connect,
                    new_auto_connect = auto_connect,
                    "Updating OBS auto_connect setting"
                );
                new_config.auto_connect = auto_connect;
            }
        }

        // Update include_scene_details if provided
        if let Some(include_details) = config
            .get("include_scene_details")
            .and_then(|v| v.as_bool())
        {
            if include_details != current_config.include_scene_details {
                info!(
                    old_value = current_config.include_scene_details,
                    new_value = include_details,
                    "Updating OBS include_scene_details setting"
                );
                new_config.include_scene_details = include_details;
            }
        }

        // Store the updated configuration
        *self.config.write().await = new_config.clone();
        debug!("Updated OBS adapter configuration");

        // If connection settings changed and we're connected, disconnect and reconnect
        if connection_settings_changed && self.base.is_connected() {
            info!(
                host = %new_config.host,
                port = new_config.port,
                "OBS connection settings changed, reconnecting"
            );

            self.disconnect().await?;

            // Wait a bit before reconnecting
            debug!(
                "Waiting {}ms before reconnecting to OBS",
                RECONNECT_DELAY_MS
            );
            sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;

            // Only reconnect if auto_connect is true
            if new_config.auto_connect {
                info!("Auto-reconnecting OBS adapter with new settings");
                self.connect().await?;
            } else {
                info!("Not auto-reconnecting OBS adapter (auto_connect is disabled)");
            }
        } else if connection_settings_changed {
            debug!("Connection settings changed but adapter not currently connected");
        } else {
            debug!("No connection-relevant settings changed");
        }

        Ok(())
    }
}

impl Clone for ObsAdapter {
    fn clone(&self) -> Self {
        // IMPORTANT: This is a proper implementation of Clone that ensures callback integrity.
        // When an adapter is cloned, it's essential that all shared state wrapped in
        // Arc is properly cloned with Arc::clone to maintain the same underlying instances.
        //
        // Common mistakes fixed here:
        // 1. Creating new RwLock/Mutex instances instead of sharing existing ones with Arc::clone
        // 2. Not properly sharing callback registries between clones
        // 3. Creating multiple copies of event buses, configs, or other shared state
        //
        // The correct pattern is to use Arc::clone for ALL fields that contain
        // state that should be shared between clones:
        // 1. BaseAdapter contains the event bus and connection state - use its clone implementation
        // 2. The client should be shared so all instances have the same connection
        // 3. The config should be shared so configuration changes affect all clones
        // 4. The callbacks registry must be shared so callbacks are maintained

        Self {
            base: self.base.clone(),          // Use BaseAdapter's clone implementation
            client: Arc::clone(&self.client), // Share the same client
            config: Arc::clone(&self.config), // Share the same config
            callbacks: Arc::clone(&self.callbacks), // Share the same callback registry
        }
    }
}

#[async_trait]
impl ServiceAdapterHelper for ObsAdapter {
    fn base(&self) -> &BaseAdapter {
        &self.base
    }

    async fn clean_up_on_disconnect(&self) -> Result<()> {
        // OBS-specific cleanup: clear the client reference
        debug!("Clearing OBS client reference");
        *self.client.lock().await = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Tests for this module are in the dedicated tests directory
    pub use crate::adapters::tests::obs_test::*;
}
