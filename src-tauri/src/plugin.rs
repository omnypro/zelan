use crate::{
    adapters::{ObsAdapter, TestAdapter, TwitchAdapter},
    auth::TokenManager,
    AdapterSettings, ErrorCode, ErrorSeverity, StreamService, ZelanError,
};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::any::Any;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use tracing::{debug, error, info, instrument, span, warn, Instrument, Level};

/// State object to store the stream service and token manager
pub struct ZelanState {
    // Using an Arc<Mutex<>> to allow for mutability behind shared ownership
    pub service: Arc<Mutex<StreamService>>,
    // Token manager for centralized authentication handling
    pub token_manager: Arc<TokenManager>,
}

// Implement Clone for ZelanState to allow cloning the service in command handlers
impl Clone for ZelanState {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            token_manager: self.token_manager.clone(),
        }
    }
}

/// Securely store sensitive tokens in the secure storage
#[instrument(skip(app, adapter_name, tokens), level = "debug")]
pub async fn store_secure_tokens(
    app: &AppHandle,
    adapter_name: &str,
    tokens: serde_json::Value,
) -> Result<()> {
    debug!(adapter = %adapter_name, "Storing secure tokens");

    // Validate the tokens before proceeding
    if tokens.is_null() || (tokens.is_object() && tokens.as_object().unwrap().is_empty()) {
        info!(adapter = %adapter_name, "No tokens to store securely");
        return Ok(());
    }

    // Create a secure store key specific to this adapter
    let secure_key = format!("secure_{}_tokens", adapter_name);

    // Get the store
    let store = app.store("zelan.secure.json").map_err(|e| {
        error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
        anyhow!("Failed to access secure store: {}", e)
    })?;

    // Store the tokens securely
    store.set(&secure_key, tokens);

    // Save the store
    if let Err(e) = store.save() {
        error!(error = %e, "Failed to save secure store");
        return Err(anyhow!("Failed to save secure store: {}", e));
    }

    info!(adapter = %adapter_name, "Tokens securely stored");
    Ok(())
}

/// Retrieve secure tokens from storage
#[instrument(skip(app, adapter_name), level = "debug")]
pub async fn retrieve_secure_tokens(
    app: &AppHandle,
    adapter_name: &str,
) -> Result<Option<serde_json::Value>> {
    debug!(adapter = %adapter_name, "Retrieving secure tokens");

    // Create a secure store key specific to this adapter
    let secure_key = format!("secure_{}_tokens", adapter_name);

    // Get the store
    let store = app.store("zelan.secure.json").map_err(|e| {
        error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
        anyhow!("Failed to access secure store: {}", e)
    })?;

    // Check if we have tokens for this adapter
    if !store.has(&secure_key) {
        debug!(adapter = %adapter_name, "No secure tokens found");
        return Ok(None);
    }

    // Get the tokens
    let tokens = store.get(&secure_key).map(|v| v.clone());

    match &tokens {
        Some(_) => debug!(adapter = %adapter_name, "Found secure tokens"),
        None => debug!(adapter = %adapter_name, "Token key exists but no value found"),
    }

    Ok(tokens)
}

/// Remove secure tokens from storage
#[instrument(skip(app, adapter_name), level = "debug")]
pub async fn remove_secure_tokens(app: &AppHandle, adapter_name: &str) -> Result<()> {
    debug!(adapter = %adapter_name, "Removing secure tokens");

    // Create a secure store key specific to this adapter
    let secure_key = format!("secure_{}_tokens", adapter_name);

    // Get the store
    let store = app.store("zelan.secure.json").map_err(|e| {
        error!(error = %e, adapter = %adapter_name, "Failed to access secure store");
        anyhow!("Failed to access secure store: {}", e)
    })?;

    // Check if tokens exist
    if !store.has(&secure_key) {
        debug!(adapter = %adapter_name, "No secure tokens to remove");
        return Ok(());
    }

    // Remove the tokens
    let deleted = store.delete(&secure_key);
    debug!(deleted = %deleted, "Deletion result");

    // Save the store to persist the deletion
    if let Err(e) = store.save() {
        error!(error = %e, "Failed to save secure store after deletion");
        return Err(anyhow!("Failed to save secure store: {}", e));
    }

    info!(adapter = %adapter_name, "Secure tokens removed");
    Ok(())
}

impl ZelanState {
    pub fn new() -> Result<Self> {
        // Create default service
        let service = StreamService::new();
        let service_arc = Arc::new(Mutex::new(service));

        // Create token manager
        let token_manager = TokenManager::new();
        let token_manager_arc = Arc::new(token_manager);

        Ok(Self {
            service: service_arc,
            token_manager: token_manager_arc,
        })
    }

    /// Export configuration
    pub async fn export_config(&self) -> Result<crate::Config> {
        // Get the current configuration and export it
        let service = self.service.lock().await;
        let config = service.export_config().await;
        Ok(config)
    }

    /// Save configuration to the store
    #[instrument(skip(self, app), level = "debug")]
    pub async fn save_config_to_store(&self, app: &AppHandle) -> Result<()> {
        // Export the current config
        let config = self.export_config().await?;
        debug!(config = ?config, "Config to be saved");

        // Process adapters that need special handling
        let mut processed_config = config.clone();
        let mut tokens_to_store = std::collections::HashMap::new();

        // Process Twitch adapter to extract tokens
        if let Some(twitch_settings) = processed_config.adapters.get_mut("twitch") {
            debug!("Processing Twitch adapter config for secure storage");

            let mut config_obj = twitch_settings
                .config
                .as_object()
                .cloned()
                .unwrap_or_default();
            let mut twitch_tokens = serde_json::Map::new();

            debug!(config_obj = ?config_obj, "Config object to inspect for tokens");

            // Check for tokens in config
            if let Some(token) = config_obj.get("access_token").cloned() {
                if !token.is_null() && token.as_str().filter(|t| !t.is_empty()).is_some() {
                    debug!("Found access_token in config, moving to secure storage");
                    twitch_tokens.insert("access_token".to_string(), token.clone());
                    // Remove from main config - use null as a placeholder
                    config_obj.insert("access_token".to_string(), serde_json::Value::Null);
                }
            }

            if let Some(token) = config_obj.get("refresh_token").cloned() {
                if !token.is_null() && token.as_str().filter(|t| !t.is_empty()).is_some() {
                    debug!("Found refresh_token in config, moving to secure storage");
                    twitch_tokens.insert("refresh_token".to_string(), token.clone());
                    // Remove from main config - use null as a placeholder
                    config_obj.insert("refresh_token".to_string(), serde_json::Value::Null);
                }
            }

            // If we have tokens to store, save them to the secure tokens map
            if !twitch_tokens.is_empty() {
                tokens_to_store.insert(
                    "twitch".to_string(),
                    serde_json::Value::Object(twitch_tokens),
                );
                // Update the config to remove the tokens
                twitch_settings.config = serde_json::Value::Object(config_obj);
            }
        }

        // Convert processed config to JSON for storing
        let config_json = json!(processed_config);
        debug!(json = %config_json, "JSON representation");

        // Get the configuration store and secure store for transaction-like operation
        let config_store = app.store("zelan.config.json").map_err(|e| {
            error!(error = %e, "Failed to access config store");
            anyhow!("Failed to access config store: {}", e)
        })?;

        // Only get secure store if we have tokens to save
        let secure_store = if !tokens_to_store.is_empty() {
            let secure_store = app.store("zelan.secure.json").map_err(|e| {
                error!(error = %e, "Failed to access secure store");
                anyhow!("Failed to access secure store: {}", e)
            })?;

            Some(secure_store)
        } else {
            None
        };

        // TRANSACTION PATTERN - Prepare all changes before committing

        // Step 1: Set up values in both stores but don't save yet
        debug!("Setting up config in config store");
        config_store.set("config", config_json);

        // Step 2: Prepare token storage if needed
        if let Some(store) = &secure_store {
            for (adapter_name, tokens) in &tokens_to_store {
                debug!(adapter = %adapter_name, "Preparing tokens for secure storage");
                let secure_key = format!("secure_{}_tokens", adapter_name);
                store.set(&secure_key, tokens.clone());
            }
        }

        // Step 3: Commit changes in a coordinated way (save both or none)
        debug!("Committing changes to stores");

        // First try to save the config store
        if let Err(e) = config_store.save() {
            error!(error = %e, "Failed to save to config store");
            return Err(anyhow!("Failed to save to config store: {}", e));
        }
        info!("Configuration successfully saved to config store");

        // Then save the secure store if needed
        if let Some(store) = &secure_store {
            if let Err(e) = store.save() {
                error!(error = %e, "Failed to save to secure store");
                warn!("Config store saved but secure store failed - data might be inconsistent");
                return Err(anyhow!("Failed to save to secure store: {}", e));
            }
            info!("Sensitive tokens successfully saved to secure store");
        }

        // All stores saved successfully
        info!("All configuration data saved successfully");
        Ok(())
    }

    /// Initialize background services
    #[instrument(skip(self, app), level = "debug")]
    pub async fn init_services(&self, app: AppHandle) -> Result<()> {
        let service = self.service.clone();
        let token_manager = self.token_manager.clone();

        // IMPORTANT: Set the app handle on the token manager FIRST
        // This must be done before any adapters try to use the token manager
        info!("Setting app handle on TokenManager");
        token_manager.set_app(app.clone()).await;
        info!("TokenManager app handle successfully set");
        
        // Ensure all tokens have expiration times set
        if let Err(e) = token_manager.ensure_twitch_token_expiration().await {
            warn!("Error ensuring Twitch token expiration: {}", e);
        } else {
            info!("Successfully ensured Twitch token expiration times");
        }

        // Pass the app handle to the initialization task
        let app_handle = app.clone();

        // We initialize services in a separate task to avoid blocking
        tauri::async_runtime::spawn(
            async move {
                // Make app available to inner functions
                let app = app_handle;
                info!("Starting background service initialization");
                
                // Get service with exclusive access
                let mut service_guard = service.lock().await;

                // First, get the event bus and create a persistent dummy subscriber to keep it alive
                let event_bus = service_guard.event_bus();
                let mut _dummy_subscriber = event_bus.subscribe();
                debug!("Created dummy event bus subscriber to keep bus alive");

                // Start the WebSocket server
                match service_guard.start_websocket_server().await {
                    Ok(_) => info!("WebSocket server started successfully"),
                    Err(e) => error!(error = %e, "Failed to start WebSocket server"),
                }

                // Start the HTTP API
                match service_guard.start_http_api().await {
                    Ok(_) => info!("HTTP API server started successfully"),
                    Err(e) => error!(error = %e, "Failed to start HTTP API server"),
                }

                // Fetch already saved adapter settings
                let adapter_settings = service_guard.get_all_adapter_settings().await;
                debug!(
                    adapter_count = adapter_settings.len(), 
                    "Found saved adapter settings"
                );
                
                // Register the test adapter with settings
                if let Some(saved_test_settings) = adapter_settings.get("test") {
                    info!(
                        enabled = saved_test_settings.enabled,
                        "Found saved Test adapter settings"
                    );
                    
                    // Create the adapter with the saved configuration if possible
                    let test_adapter = match TestAdapter::config_from_json(&saved_test_settings.config) {
                        Ok(config) => {
                            info!(
                                interval_ms = config.interval_ms,
                                generate_special = config.generate_special_events,
                                "Using saved Test adapter configuration"
                            );
                            TestAdapter::with_config(service_guard.event_bus(), config)
                        },
                        Err(e) => {
                            warn!(
                                error = %e,
                                "Failed to parse saved Test adapter config, using defaults"
                            );
                            TestAdapter::new(service_guard.event_bus())
                        }
                    };
                    
                    // Register with saved settings
                    service_guard.register_adapter(test_adapter, Some(saved_test_settings.clone())).await;
                    info!("Registered Test adapter with saved settings");
                } else {
                    info!("No saved Test adapter settings found, using defaults");
                    
                    // Import the TestConfig type and AdapterConfig trait
                    use crate::adapters::{base::AdapterConfig, test::TestConfig};
                    
                    // Create a default test adapter
                    let test_adapter = TestAdapter::new(service_guard.event_bus());
                    
                    // Define default settings with the proper config
                    let default_config = TestConfig::default();
                    let test_settings = AdapterSettings {
                        enabled: true,
                        config: default_config.to_json(),
                        display_name: "Test Adapter".to_string(),
                        description: "A test adapter that generates sample events at regular intervals for development and debugging".to_string(),
                    };
                    
                    // Register the adapter
                    service_guard
                        .register_adapter(test_adapter, Some(test_settings))
                        .await;
                    debug!("Registered Test adapter with default settings");
                }

            // Register the OBS adapter with settings
            if let Some(saved_obs_settings) = adapter_settings.get("obs") {
                info!(
                    enabled = saved_obs_settings.enabled,
                    "Found saved OBS adapter settings"
                );
                
                // Extract the saved OBS configuration from the settings
                let config = if let Some(config_value) = saved_obs_settings.config.as_object() {
                    // Import the OBS adapter config from the adapters module
                    use crate::adapters::obs::ObsConfig;
                    
                    // Create an OBS config from the saved settings
                    let mut obs_config = ObsConfig::default();
                    
                    // Extract the host (use default if not present)
                    if let Some(host) = config_value.get("host").and_then(|v| v.as_str()) {
                        obs_config.host = host.to_string();
                    }
                    
                    // Extract the port (use default if not present)
                    if let Some(port) = config_value.get("port").and_then(|v| v.as_u64()) {
                        obs_config.port = port as u16;
                    }
                    
                    // Extract the password (use default if not present)
                    if let Some(password) = config_value.get("password").and_then(|v| v.as_str()) {
                        obs_config.password = password.to_string();
                    }
                    
                    // Extract auto_connect (use default if not present)
                    if let Some(auto_connect) = config_value.get("auto_connect").and_then(|v| v.as_bool()) {
                        obs_config.auto_connect = auto_connect;
                    }
                    
                    // Extract include_scene_details (use default if not present)
                    if let Some(include_details) = config_value.get("include_scene_details").and_then(|v| v.as_bool()) {
                        obs_config.include_scene_details = include_details;
                    }
                    
                    obs_config
                } else {
                    // Import the OBS adapter config from the adapters module
                    use crate::adapters::obs::ObsConfig;
                    warn!("No valid OBS configuration found, using defaults");
                    ObsConfig::default()
                };
                
                // Create the adapter with the loaded configuration
                info!(host = %config.host, port = config.port, "Initializing OBS adapter");
                let obs_adapter = ObsAdapter::with_config(service_guard.event_bus(), config);
                
                // Register the adapter with its settings
                service_guard.register_adapter(obs_adapter, Some(saved_obs_settings.clone())).await;
                info!("Registered OBS adapter with saved settings");
            } else {
                info!("No saved OBS settings found, using defaults");
                let obs_adapter = ObsAdapter::new(service_guard.event_bus());
                let obs_settings = AdapterSettings {
                    enabled: true,
                    config: serde_json::json!({
                        "host": "localhost",
                        "port": 4455,
                        "password": "",
                        "auto_connect": true,
                        "include_scene_details": true
                    }),
                    display_name: "OBS Studio".to_string(),
                    description: "Connects to OBS Studio via WebSocket to receive scene changes, streaming status, and other events".to_string(),
                };
                service_guard
                    .register_adapter(obs_adapter, Some(obs_settings))
                    .await;
                debug!("Registered OBS adapter with default settings");
            }
            
            // Register the Twitch adapter with settings
            if let Some(saved_twitch_settings) = adapter_settings.get("twitch") {
                info!(
                    enabled = saved_twitch_settings.enabled,
                    "Found saved Twitch adapter settings"
                );
                
                // Extract the saved Twitch configuration from the settings
                let mut config = if let Some(config_value) = saved_twitch_settings.config.as_object() {
                    // Import the Twitch adapter config from the adapters module
                    use crate::adapters::twitch::TwitchConfig;
                    
                    // Create a Twitch config from the saved settings
                    let mut twitch_config = TwitchConfig::default();
                    
                    // Extract the channel_id (use default if not present)
                    if let Some(channel_id) = config_value.get("channel_id").and_then(|v| v.as_str()) {
                        twitch_config.channel_id = Some(channel_id.to_string());
                    }
                    
                    // Extract the channel_login (use default if not present)
                    if let Some(channel_login) = config_value.get("channel_login").and_then(|v| v.as_str()) {
                        twitch_config.channel_login = Some(channel_login.to_string());
                    }
                    
                    // Extract the access_token (if present)
                    if let Some(access_token) = config_value.get("access_token").and_then(|v| v.as_str()) {
                        if !access_token.is_empty() {
                            twitch_config.access_token = Some(access_token.to_string());
                        }
                    }
                    
                    // Extract the refresh_token (if present)
                    if let Some(refresh_token) = config_value.get("refresh_token").and_then(|v| v.as_str()) {
                        if !refresh_token.is_empty() {
                            twitch_config.refresh_token = Some(refresh_token.to_string());
                        }
                    }
                    
                    // Extract poll_interval_ms (use default if not present)
                    if let Some(poll_interval) = config_value.get("poll_interval_ms").and_then(|v| v.as_u64()) {
                        twitch_config.poll_interval_ms = poll_interval;
                    }
                    
                    // Extract monitor_channel_info (use default if not present)
                    if let Some(monitor_channel) = config_value.get("monitor_channel_info").and_then(|v| v.as_bool()) {
                        twitch_config.monitor_channel_info = monitor_channel;
                    }
                    
                    twitch_config
                } else {
                    // Import the Twitch adapter config from the adapters module
                    use crate::adapters::twitch::TwitchConfig;
                    warn!("No valid Twitch configuration found, using defaults");
                    TwitchConfig::default()
                };
                
                // Try to load secure tokens if they exist
                if config.access_token.is_none() || config.refresh_token.is_none() {
                    // Use the app handle passed from the init method
                    {
                        // We're in a separate task flow, so clone the app handle to move it
                        let app_handle = app.clone();
                        // See if we have secure tokens stored
                        match retrieve_secure_tokens(&app_handle, "twitch").await {
                            Ok(Some(tokens)) => {
                                debug!("Found secure tokens for Twitch");
                                
                                // Add tokens back for adapter configuration
                                if let Some(token_obj) = tokens.as_object() {
                                    // Check for access token
                                    if let Some(token) = token_obj.get("access_token") {
                                        if token.is_string() {
                                            debug!("Loading access_token from secure storage");
                                            config.access_token = Some(token.as_str().unwrap().to_string());
                                        }
                                    }
                                    
                                    // Check for refresh token
                                    if let Some(token) = token_obj.get("refresh_token") {
                                        if token.is_string() {
                                            debug!("Loading refresh_token from secure storage");
                                            config.refresh_token = Some(token.as_str().unwrap().to_string());
                                        }
                                    }
                                }
                                
                                info!("Loaded secure Twitch tokens from storage");
                            },
                            Ok(None) => debug!("No secure tokens found for Twitch"),
                            Err(e) => warn!(error = %e, "Failed to retrieve secure tokens"),
                        }
                    }
                }
                
                // Create the adapter with the loaded configuration
                info!("Initializing Twitch adapter");
                let mut twitch_adapter = TwitchAdapter::with_config(service_guard.event_bus(), config);
                
                // Set token manager on the adapter - IMPORTANT: This must happen before registering
                // to ensure the token manager is available when connect() is called
                info!("Setting TokenManager on Twitch adapter");
                twitch_adapter.set_token_manager(Arc::clone(&service_guard.token_manager));
                
                // Set recovery manager
                info!("Setting RecoveryManager on Twitch adapter");
                twitch_adapter.set_recovery_manager(service_guard.recovery_manager.clone());
                
                info!("TokenManager successfully set on Twitch adapter");
                
                // Register the adapter with its settings
                service_guard.register_adapter(twitch_adapter, Some(saved_twitch_settings.clone())).await;
                info!("Registered Twitch adapter with saved settings");
            } else {
                info!("No saved Twitch settings found, using defaults");
                let mut twitch_adapter = TwitchAdapter::new(service_guard.event_bus());
                
                // Set token manager on the adapter - IMPORTANT: This must happen before registering
                // to ensure the token manager is available when connect() is called
                info!("Setting TokenManager on Twitch adapter");
                twitch_adapter.set_token_manager(Arc::clone(&service_guard.token_manager));
                
                // Set recovery manager
                info!("Setting RecoveryManager on Twitch adapter");
                twitch_adapter.set_recovery_manager(service_guard.recovery_manager.clone());
                
                info!("TokenManager successfully set on Twitch adapter");
                
                let twitch_settings = AdapterSettings {
                    enabled: true,
                    config: serde_json::json!({
                        "channel_id": null,
                        "channel_login": null,
                        "poll_interval_ms": 30000, 
                        "monitor_channel_info": true
                    }),
                    display_name: "Twitch".to_string(),
                    description: "Connects to Twitch to receive stream information and events".to_string(),
                };
                service_guard
                    .register_adapter(twitch_adapter, Some(twitch_settings))
                    .await;
                debug!("Registered Twitch adapter with default settings");
            }
            
            // Connect all adapters
            match service_guard.connect_all_adapters().await {
                Ok(_) => info!("All adapters connected successfully"),
                Err(e) => error!(error = %e, "Failed to connect adapters"),
            }

            info!("Zelan background services started successfully");

            // Release the lock on the service
            drop(service_guard);

            // Keep a long-running task to process events and maintain the event bus
            loop {
                // Process the dummy subscription to keep it alive
                match _dummy_subscriber.recv().await {
                    Ok(event) => {
                        // Check for special save_tokens_request event
                        if event.event_type() == "save_tokens_request" && event.source() == "system" {
                            info!("Received request to save tokens for an adapter");
                            
                            // Extract the tokens and adapter name
                            let payload = event.payload();
                            if let (Some(adapter), Some(access_token)) = (
                                payload.get("adapter").and_then(|v| v.as_str()),
                                payload.get("access_token").and_then(|v| v.as_str())
                            ) {
                                let refresh_token = payload.get("refresh_token")
                                    .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) });
                                
                                info!(adapter = %adapter, "Processing token save request for adapter");
                                
                                // Clone the refresh token to avoid ownership issues
                                let refresh_token_clone = refresh_token.clone();
                                
                                // Create TokenData
                                use crate::auth::token_manager::TokenData;
                                let token_data = TokenData::new(
                                    access_token.to_string(),
                                    refresh_token_clone
                                );
                                
                                // Get token manager
                                let token_manager = token_manager.clone();
                                
                                // Store tokens using token manager
                                match token_manager.store_tokens(adapter, token_data).await {
                                    Ok(_) => {
                                        info!("Successfully saved tokens to TokenManager for {}", adapter);
                                    }
                                    Err(e) => {
                                        error!("Failed to save tokens to TokenManager: {}", e);
                                        
                                        // Fallback to legacy method
                                        let token_json = json!({
                                            "access_token": access_token,
                                            "refresh_token": refresh_token
                                        });
                                        
                                        if let Err(e) = store_secure_tokens(&app, adapter, token_json).await {
                                            error!("Fallback token storage also failed: {}", e);
                                        }
                                    }
                                };
                            }
                        } else {
                            debug!(
                                source = %event.source(),
                                event_type = %event.event_type(),
                                "Background listener received event"
                            );
                        }
                    }
                    Err(e) => {
                        // Only log errors that are not lagged messages
                        if !e.to_string().contains("lagged") {
                            error!(error = %e, "Background listener error");
                        }
                    }
                }
            }
            }
            .instrument(span!(Level::INFO, "service_initialization"))
        );

        Ok(())
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde_json;

/// Get current status of the event bus
#[tauri::command]
pub async fn get_event_bus_status(
    state: State<'_, ZelanState>,
) -> Result<serde_json::Value, ZelanError> {
    // Get the event bus with the async mutex
    let service = state.service.lock().await;
    let event_bus = service.event_bus().clone();

    // Now we can await outside the lock scope
    let stats = event_bus.get_stats().await;

    // Convert to JSON value
    serde_json::to_value(stats).map_err(|e| ZelanError {
        code: ErrorCode::Internal,
        message: "Failed to serialize event bus stats".to_string(),
        context: Some(e.to_string()),
        severity: ErrorSeverity::Error,
        category: Some(crate::ErrorCategory::Internal),
        error_id: None,
    })
}

/// Get all adapter statuses
#[tauri::command]
pub async fn get_adapter_statuses(
    state: State<'_, ZelanState>,
) -> Result<serde_json::Value, ZelanError> {
    // Get the service with the async mutex
    let service_guard = state.service.lock().await;
    let service_clone = service_guard.clone();

    // Now we can safely await without holding the lock
    let statuses = service_clone.get_all_statuses().await;

    // Convert to JSON value
    serde_json::to_value(statuses).map_err(|e| ZelanError {
        code: ErrorCode::Internal,
        message: "Failed to serialize adapter statuses".to_string(),
        context: Some(e.to_string()),
        severity: ErrorSeverity::Error,
        category: Some(crate::ErrorCategory::Internal),
        error_id: None,
    })
}

/// Get all adapter settings
#[tauri::command]
pub async fn get_adapter_settings(
    state: State<'_, ZelanState>,
) -> Result<serde_json::Value, ZelanError> {
    // Get the service with the async mutex
    let service_guard = state.service.lock().await;
    let service_clone = service_guard.clone();

    // Now we can safely await without holding the lock
    let settings = service_clone.get_all_adapter_settings().await;

    // Convert to JSON value
    serde_json::to_value(settings).map_err(|e| ZelanError {
        code: ErrorCode::Internal,
        message: "Failed to serialize adapter settings".to_string(),
        context: Some(e.to_string()),
        severity: ErrorSeverity::Error,
        category: Some(crate::ErrorCategory::Internal),
        error_id: None,
    })
}

/// Update adapter settings
#[tauri::command]
#[instrument(skip(app, settings, state), fields(adapter_name = %adapter_name), level = "debug")]
pub async fn update_adapter_settings(
    app: AppHandle,
    adapter_name: String,
    settings: serde_json::Value,
    state: State<'_, ZelanState>,
) -> Result<String, ZelanError> {
    info!(adapter = %adapter_name, "Updating adapter settings");

    // Deserialize the settings
    let mut adapter_settings: AdapterSettings = match serde_json::from_value(settings.clone()) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Invalid adapter settings format");
            return Err(ZelanError {
                code: ErrorCode::ConfigInvalid,
                message: "Invalid adapter settings format".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
                category: Some(crate::ErrorCategory::Configuration),
                error_id: None,
            });
        }
    };

    // Special handling for Twitch adapter - extract and store tokens securely
    if adapter_name == "twitch" {
        debug!("Processing Twitch adapter with secure token handling");

        // Log the raw settings to understand what's coming in
        debug!(settings = ?adapter_settings, "Raw adapter settings for inspection");

        // Extract tokens from config if they exist
        let mut config_obj = adapter_settings
            .config
            .as_object()
            .cloned()
            .unwrap_or_default();

        debug!(config_obj = ?config_obj, "Config object to inspect for tokens");
        let mut tokens_to_store = serde_json::Map::new();

        // Check if access_token and refresh_token are in the config
        if let Some(token) = config_obj.get("access_token").cloned() {
            debug!(access_token = ?token, "Checking access token value");

            // Log more specific details about the token
            if token.is_null() {
                debug!("Access token is null");
            } else if token.is_string() {
                let token_str = token.as_str().unwrap_or("INVALID_STRING");
                debug!(token_str, "Access token string value");
                if token_str.is_empty() {
                    debug!("Access token is an empty string");
                }
            } else {
                debug!(token_type = ?token.type_id(), "Access token is not a string or null");
            }

            if !token.is_null() && token.as_str().filter(|t| !t.is_empty()).is_some() {
                debug!("Found access_token in config, moving to secure storage");
                tokens_to_store.insert("access_token".to_string(), token.clone());
                // Remove from main config - use null as a placeholder
                config_obj.insert("access_token".to_string(), serde_json::Value::Null);
            }
        } else {
            debug!("No access_token key found in config");
        }

        if let Some(token) = config_obj.get("refresh_token").cloned() {
            debug!(refresh_token = ?token, "Checking refresh token value");

            // Log more specific details about the token
            if token.is_null() {
                debug!("Refresh token is null");
            } else if token.is_string() {
                let token_str = token.as_str().unwrap_or("INVALID_STRING");
                debug!(token_str, "Refresh token string value");
                if token_str.is_empty() {
                    debug!("Refresh token is an empty string");
                }
            } else {
                debug!(token_type = ?token.type_id(), "Refresh token is not a string or null");
            }

            if !token.is_null() && token.as_str().filter(|t| !t.is_empty()).is_some() {
                debug!("Found refresh_token in config, moving to secure storage");
                tokens_to_store.insert("refresh_token".to_string(), token.clone());
                // Remove from main config - use null as a placeholder
                config_obj.insert("refresh_token".to_string(), serde_json::Value::Null);
            }
        } else {
            debug!("No refresh_token key found in config");
        }

        // Store tokens securely if we have any
        if !tokens_to_store.is_empty() {
            debug!("Storing Twitch tokens securely");
            if let Err(e) = store_secure_tokens(
                &app,
                &adapter_name,
                serde_json::Value::Object(tokens_to_store.clone()),
            )
            .await
            {
                error!(error = %e, "Failed to store tokens securely");
                // Don't return error, continue with non-secure storage as fallback
                // But log a warning
                warn!("Falling back to non-secure token storage");
            } else {
                info!("Twitch tokens stored securely");
                // Update the config object without the sensitive tokens
                adapter_settings.config = serde_json::Value::Object(config_obj);
            }
        } else {
            debug!("No tokens to store securely");
        }
    }

    // Get the service with the async mutex
    let service = state.service.lock().await;
    let service_clone = service.clone();

    // Release the mutex lock before async operations
    drop(service);

    // Update the settings
    debug!(
        adapter = %adapter_name,
        enabled = adapter_settings.enabled,
        "Updating adapter settings"
    );

    match service_clone
        .update_adapter_settings(&adapter_name, adapter_settings.clone())
        .await
    {
        Ok(_) => {
            info!(adapter = %adapter_name, "Successfully updated adapter settings");

            // For Twitch, check for secure tokens and provide them to the adapter if needed
            if adapter_name == "twitch" {
                debug!("Checking for secure Twitch tokens");

                // See if we have secure tokens stored
                match retrieve_secure_tokens(&app, &adapter_name).await {
                    Ok(Some(tokens)) => {
                        info!("Found secure tokens for Twitch, applying to adapter");
                        debug!(tokens = ?tokens, "Secure tokens to apply");

                        // Create a config with the secure tokens included
                        let mut config = adapter_settings
                            .config
                            .as_object()
                            .cloned()
                            .unwrap_or_default();

                        debug!(before_tokens = ?config, "Config before adding tokens");

                        // Add tokens back for adapter configuration
                        if let Some(token_obj) = tokens.as_object() {
                            for (key, value) in token_obj {
                                if key == "access_token" || key == "refresh_token" {
                                    info!("Adding {} from secure storage to adapter config", key);
                                    if value.is_string() {
                                        let token_val = value.as_str().unwrap_or("");
                                        debug!(token_length = token_val.len(), "Token length");
                                    }
                                    config.insert(key.clone(), value.clone());
                                }
                            }
                        }

                        // Apply the complete config with secure tokens to the adapter
                        let full_config = serde_json::Value::Object(config.clone());
                        debug!(after_tokens = ?config, "Config after adding tokens");

                        // Let the adapter handle the tokens (don't error on failure)
                        if let Err(e) = service_clone
                            .update_adapter_settings(
                                &adapter_name,
                                AdapterSettings {
                                    enabled: adapter_settings.enabled,
                                    config: full_config,
                                    display_name: adapter_settings.display_name.clone(),
                                    description: adapter_settings.description.clone(),
                                },
                            )
                            .await
                        {
                            warn!(error = %e, "Failed to apply secure tokens to adapter");
                        } else {
                            info!("Successfully applied secure tokens to Twitch adapter");
                        }
                    }
                    Ok(None) => debug!("No secure tokens found for Twitch"),
                    Err(e) => warn!(error = %e, "Failed to retrieve secure tokens"),
                }
            }

            // For Twitch, monitor for tokens provided directly by the auth flow
            if adapter_name == "twitch" && adapter_settings.enabled {
                // Start a background task to periodically check if tokens appear that need to be secured
                let adapter_name_clone = adapter_name.clone();
                let service_clone2 = service_clone.clone();

                tauri::async_runtime::spawn(async move {
                    // Wait a bit for authentication to complete if it's ongoing
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    // Check adapter settings for tokens
                    if let Ok(current_settings) = service_clone2
                        .get_adapter_settings(&adapter_name_clone)
                        .await
                    {
                        if let Some(config_obj) = current_settings.config.as_object() {
                            // Check for access token
                            let has_token = config_obj
                                .get("access_token")
                                .filter(|t| {
                                    !t.is_null() && t.as_str().filter(|s| !s.is_empty()).is_some()
                                })
                                .is_some();

                            if has_token {
                                info!("Found access token in Twitch adapter after delay, should be moved to secure storage");
                                // Next settings update will handle this automatically
                            }
                        }
                    }
                });
            }

            // If adapter is enabled, explicitly try to connect it
            if adapter_settings.enabled {
                info!(adapter = %adapter_name, "Adapter is enabled, attempting connection");

                match service_clone.connect_adapter(&adapter_name).await {
                    Ok(_) => info!(adapter = %adapter_name, "Successfully connected adapter"),
                    Err(connect_err) => {
                        warn!(
                            error = %connect_err,
                            adapter = %adapter_name,
                            "Failed to connect adapter after settings update"
                        );
                        // We'll continue and save settings anyway
                    }
                }
            }

            // Get the current config for debugging
            match state.export_config().await {
                Ok(config) => debug!(config = ?config, "Current config to save"),
                Err(e) => warn!(error = %e, "Failed to export config for debugging"),
            }

            // Save the current config to the store
            if let Err(e) = state.save_config_to_store(&app).await {
                error!(
                    error = %e,
                    adapter = %adapter_name,
                    "Failed to save configuration after updating adapter settings"
                );
                return Err(ZelanError {
                    code: ErrorCode::Internal,
                    message: "Failed to save configuration".to_string(),
                    context: Some(e.to_string()),
                    severity: ErrorSeverity::Warning,
                    category: Some(crate::ErrorCategory::Internal),
                    error_id: None,
                });
            } else {
                info!("Successfully saved config to store");
            }

            // Success, log completion
            info!(adapter = %adapter_name, "Settings successfully updated and saved");
        }
        Err(e) => {
            error!(error = %e, adapter = %adapter_name, "Failed to update adapter settings");
            return Err(e);
        }
    }

    Ok(format!(
        "Successfully updated settings for adapter '{}'",
        adapter_name
    ))
}

/// Send a test event through the system
#[tauri::command]
pub async fn send_test_event(state: State<'_, ZelanState>) -> Result<String, ZelanError> {
    use crate::StreamEvent;
    use serde_json::json;

    // Get the event bus from the service with async mutex
    let service = state.service.lock().await;
    let event_bus = service.event_bus().clone();

    // Create a manual test event with more details
    let event = StreamEvent::new(
        "manual",
        "test.manual",
        json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "message": "Manual test event from frontend",
            "source": "UI",
            "id": uuid::Uuid::new_v4().to_string()
        }),
    );

    // Attempt to publish the event
    match event_bus.publish(event).await {
        Ok(receivers) => Ok(format!(
            "Test event sent successfully to {} receivers",
            receivers
        )),
        Err(e) => Err(e),
    }
}

/// Get current WebSocket connection info
#[tauri::command]
pub async fn get_websocket_info(
    state: State<'_, ZelanState>,
) -> Result<serde_json::Value, ZelanError> {
    // Get the WebSocket configuration with async mutex
    let service = state.service.lock().await;
    let config = service.ws_config().clone();

    // Build info object with the WebSocket URI and helpful tips
    let info = serde_json::json!({
        "port": config.port,
        "uri": format!("ws://127.0.0.1:{}", config.port),
        "httpUri": format!("http://127.0.0.1:{}", config.port + 1),
        "wscat": format!("wscat -c ws://127.0.0.1:{}", config.port),
        "websocat": format!("websocat ws://127.0.0.1:{}", config.port),
    });

    Ok(info)
}

/// Update WebSocket port configuration
#[tauri::command]
pub async fn set_websocket_port(
    app: AppHandle,
    port: u16,
    state: State<'_, ZelanState>,
) -> Result<String, ZelanError> {
    // Validate port number
    if port < 1024 {
        return Err(ZelanError {
            code: ErrorCode::ConfigInvalid,
            message: "Invalid port number".to_string(),
            context: Some(format!("Port must be between 1024 and 65535, got {}", port)),
            severity: ErrorSeverity::Error,
            category: Some(crate::ErrorCategory::Configuration),
            error_id: None,
        });
    }

    // Get the service and immediately modify its configuration
    {
        let mut service_guard = state.service.lock().await;
        let mut config = service_guard.ws_config().clone();
        config.port = port;
        service_guard.set_ws_config(config);
        // Guard is dropped at end of this block
    }

    // Save the current config to the store
    if let Err(e) = state.save_config_to_store(&app).await {
        eprintln!(
            "Failed to save configuration after updating WebSocket port: {}",
            e
        );
        return Err(ZelanError {
            code: ErrorCode::Internal,
            message: "Failed to save configuration".to_string(),
            context: Some(e.to_string()),
            severity: ErrorSeverity::Warning,
            category: Some(crate::ErrorCategory::Internal),
            error_id: None,
        });
    }

    println!("WebSocket port successfully updated to {}", port);

    // Note: This doesn't restart the server, you'll need to restart the app
    // for the port change to take effect

    Ok(format!(
        "WebSocket port updated to {}. Restart the application for changes to take effect.",
        port
    ))
}

/// Connect an adapter by name
#[tauri::command]
#[instrument(skip(state), fields(adapter_name = %adapter_name), level = "debug")]
pub async fn connect_adapter(
    state: State<'_, ZelanState>,
    adapter_name: String,
) -> Result<String, ZelanError> {
    debug!("Connecting adapter: {}", adapter_name);

    // Access the service
    let service = state.service.lock().await;

    // Connect the adapter
    match service.connect_adapter(&adapter_name).await {
        Ok(_) => {
            info!("Successfully connected adapter: {}", adapter_name);
            Ok(format!("Connected adapter: {}", adapter_name))
        }
        Err(e) => {
            error!(error = %e, "Failed to connect adapter: {}", adapter_name);
            Err(e)
        }
    }
}

/// Disconnect an adapter by name
#[tauri::command]
#[instrument(skip(state), fields(adapter_name = %adapter_name), level = "debug")]
pub async fn disconnect_adapter(
    state: State<'_, ZelanState>,
    adapter_name: String,
) -> Result<String, ZelanError> {
    debug!("Disconnecting adapter: {}", adapter_name);

    // Access the service
    let service = state.service.lock().await;

    // Disconnect the adapter
    match service.disconnect_adapter(&adapter_name).await {
        Ok(_) => {
            info!("Successfully disconnected adapter: {}", adapter_name);
            Ok(format!("Disconnected adapter: {}", adapter_name))
        }
        Err(e) => {
            error!(error = %e, "Failed to disconnect adapter: {}", adapter_name);
            Err(e)
        }
    }
}

/// Get authentication status for an adapter
#[tauri::command]
#[instrument(skip(app, state), fields(adapter_name = %adapter_name), level = "debug")]
pub async fn get_adapter_auth_status(
    app: AppHandle,
    state: State<'_, ZelanState>,
    adapter_name: String,
) -> Result<serde_json::Value, ZelanError> {
    debug!("Getting auth status for adapter: {}", adapter_name);

    // Access the service
    let service = state.service.lock().await;

    // Check if we have settings for this adapter
    let settings = match service.get_adapter_settings(&adapter_name).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to get settings for adapter: {}", adapter_name);
            return Err(ZelanError {
                code: ErrorCode::AdapterNotFound,
                message: format!("Adapter '{}' not found", adapter_name),
                context: None,
                severity: ErrorSeverity::Error,
                category: Some(crate::ErrorCategory::NotFound),
                error_id: None,
            });
        }
    };

    // For Twitch adapter, check if we have access token in config
    if adapter_name == "twitch" {
        // Check if there's an access token in the settings
        let has_token = settings
            .config
            .get("access_token")
            .and_then(|t| t.as_str())
            .filter(|s| !s.is_empty())
            .is_some();

        // Check for token in secure storage
        let tokens = match retrieve_secure_tokens(&app, &adapter_name).await {
            Ok(Some(tokens)) => tokens,
            _ => serde_json::json!(null),
        };

        let has_secure_token = tokens
            .get("access_token")
            .and_then(|t| t.as_str())
            .filter(|s| !s.is_empty())
            .is_some();

        return Ok(serde_json::json!({
            "adapter": adapter_name,
            "has_token": has_token || has_secure_token,
            "is_authenticated": has_token || has_secure_token,
            "auth_method": "device_code",
            "scopes": ["user:read:email", "channel:read:subscriptions"],
        }));
    }

    // Default response for other adapters
    Ok(serde_json::json!({
        "adapter": adapter_name,
        "has_token": false,
        "is_authenticated": true, // Most other adapters don't require auth
        "auth_method": "none",
    }))
}

/// Initialize the Zelan state
pub fn init_state() -> ZelanState {
    match ZelanState::new() {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to create ZelanState: {}", e);
            panic!("Could not initialize Zelan state");
        }
    }
}
