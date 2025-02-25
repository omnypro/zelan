use crate::{
    adapters::{self, ObsAdapter, TestAdapter},
    AdapterSettings, ErrorCode, ErrorSeverity, StreamService, ZelanError,
};
use serde_json::json;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

/// State object to store the stream service
pub struct ZelanState {
    // Using an Arc<Mutex<>> to allow for mutability behind shared ownership
    pub service: Arc<Mutex<StreamService>>,
}

// Implement Clone for ZelanState to allow cloning the service in command handlers
impl Clone for ZelanState {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
        }
    }
}

impl ZelanState {
    pub fn new() -> Result<Self> {
        // Create default service
        let service = StreamService::new();
        let service_arc = Arc::new(Mutex::new(service));

        Ok(Self {
            service: service_arc,
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
    pub async fn save_config_to_store(&self, app: &AppHandle) -> Result<()> {
        // Export the current config
        let config = self.export_config().await?;
        println!("Config to be saved: {:?}", config);
        
        // Convert to JSON for storing
        let config_json = json!(config);
        println!("JSON representation: {}", config_json);
        
        // Get the store
        if let Ok(store) = app.store("zelan.config.json") {
            println!("Got store reference, setting config");
            
            // Save to store
            store.set("config".to_string(), config_json);
            println!("Config set in store, saving...");
            
            store.save().map_err(|e| anyhow!("Failed to save to store: {}", e))?;
            
            println!("Configuration successfully saved to store");
            return Ok(());
        }
        
        // If we get here, we couldn't access the store
        println!("Failed to get store reference");
        Err(anyhow!("Failed to access store"))
    }

    /// Initialize background services
    pub async fn init_services(&self) -> Result<()> {
        let service = self.service.clone();

        // We initialize services in a separate task to avoid blocking
        tokio::spawn(async move {
            // Get service with exclusive access
            let mut service_guard = service.lock().await;

            // First, get the event bus and create a persistent dummy subscriber to keep it alive
            let event_bus = service_guard.event_bus();
            let mut _dummy_subscriber = event_bus.subscribe();

            // Start the WebSocket server
            if let Err(e) = service_guard.start_websocket_server().await {
                eprintln!("Failed to start WebSocket server: {}", e);
            }

            // Start the HTTP API
            if let Err(e) = service_guard.start_http_api().await {
                eprintln!("Failed to start HTTP API: {}", e);
            }

            // Fetch already saved adapter settings
            let adapter_settings = service_guard.get_all_adapter_settings().await;
            
            // Register the test adapter with settings
            if let Some(saved_test_settings) = adapter_settings.get("test") {
                println!("Found saved Test adapter settings: {:?}", saved_test_settings);
                
                // Register with saved settings
                let test_adapter = TestAdapter::new(service_guard.event_bus());
                service_guard.register_adapter(test_adapter, Some(saved_test_settings.clone())).await;
                println!("Registered Test adapter with saved settings");
            } else {
                println!("No saved Test adapter settings found, using defaults");
                let test_adapter = TestAdapter::new(service_guard.event_bus());
                let test_settings = AdapterSettings {
                    enabled: true,
                    config: serde_json::json!({
                        "interval_ms": 1000, // Generate events every second
                        "generate_special_events": true,
                    }),
                    display_name: "Test Adapter".to_string(),
                    description: "A test adapter that generates sample events at regular intervals for development and debugging".to_string(),
                };
                service_guard
                    .register_adapter(test_adapter, Some(test_settings))
                    .await;
            }

            // Register the OBS adapter with settings
            if let Some(saved_obs_settings) = adapter_settings.get("obs") {
                println!("Found saved OBS settings: {:?}", saved_obs_settings);
                
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
                    ObsConfig::default()
                };
                
                // Create the adapter with the loaded configuration
                println!("Initializing OBS adapter with host: {}, port: {}", config.host, config.port);
                let obs_adapter = ObsAdapter::with_config(service_guard.event_bus(), config);
                
                // Register the adapter with its settings
                service_guard.register_adapter(obs_adapter, Some(saved_obs_settings.clone())).await;
                println!("Registered OBS adapter with saved settings");
            } else {
                println!("No saved OBS settings found, using defaults");
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
            }
            
            // Connect all adapters
            if let Err(e) = service_guard.connect_all_adapters().await {
                eprintln!("Failed to connect adapters: {}", e);
            }

            println!("Zelan background services started successfully!");

            // Release the lock on the service
            drop(service_guard);

            // Keep a long-running task to process events and maintain the event bus
            loop {
                // Process the dummy subscription to keep it alive
                match _dummy_subscriber.recv().await {
                    Ok(event) => {
                        println!(
                            "Background listener received event: {}.{}",
                            event.source(),
                            event.event_type()
                        );
                    }
                    Err(e) => {
                        // Only log errors that are not lagged messages
                        if !e.to_string().contains("lagged") {
                            eprintln!("Background listener error: {}", e);
                        }
                    }
                }
            }
        });

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
    })
}

/// Update adapter settings
#[tauri::command]
pub async fn update_adapter_settings(
    app: AppHandle,
    adapterName: String,
    settings: serde_json::Value,
    state: State<'_, ZelanState>,
) -> Result<String, ZelanError> {
    // Deserialize the settings
    let adapter_settings: AdapterSettings = match serde_json::from_value(settings) {
        Ok(s) => s,
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::ConfigInvalid,
                message: "Invalid adapter settings format".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

    // Get the service with the async mutex
    let service = state.service.lock().await;
    let service_clone = service.clone();

    // Release the mutex lock before async operations
    drop(service);

    // Update the settings
    println!(
        "Updating adapter settings for {} with enabled={}",
        adapterName, adapter_settings.enabled
    );
    match service_clone
        .update_adapter_settings(&adapterName, adapter_settings.clone())
        .await
    {
        Ok(_) => {
            println!("Successfully updated adapter settings for {}", adapterName);

            // If adapter is enabled, explicitly try to connect it
            if adapter_settings.enabled {
                println!(
                    "Adapter {} is enabled, attempting to connect...",
                    adapterName
                );
                if let Err(connect_err) = service_clone.connect_adapter(&adapterName).await {
                    println!(
                        "Warning: Failed to connect adapter after settings update: {}",
                        connect_err
                    );
                    // We'll continue and save settings anyway
                } else {
                    println!("Successfully connected adapter {}", adapterName);
                }
            }

            // Get the current config for debugging
            match state.export_config().await {
                Ok(config) => println!("Current config to save: {:?}", config),
                Err(e) => println!("Failed to export config for debugging: {}", e),
            }
            
            // Save the current config to the store
            if let Err(e) = state.save_config_to_store(&app).await {
                eprintln!("Failed to save configuration after updating adapter settings: {}", e);
                return Err(ZelanError {
                    code: ErrorCode::Internal,
                    message: "Failed to save configuration".to_string(),
                    context: Some(e.to_string()),
                    severity: ErrorSeverity::Warning,
                });
            } else {
                println!("Successfully saved config to store");
            }

            // Success, return ok
            println!(
                "Settings successfully updated and saved for {}",
                adapterName
            );
        }
        Err(e) => {
            eprintln!("Failed to update adapter settings: {}", e);
            return Err(e);
        }
    }

    Ok(format!(
        "Successfully updated settings for adapter '{}'",
        adapterName
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
        eprintln!("Failed to save configuration after updating WebSocket port: {}", e);
        return Err(ZelanError {
            code: ErrorCode::Internal,
            message: "Failed to save configuration".to_string(),
            context: Some(e.to_string()),
            severity: ErrorSeverity::Warning,
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
