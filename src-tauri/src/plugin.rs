use crate::{
    adapters::TestAdapter, AdapterSettings, ErrorCode, ErrorSeverity, StreamService, ZelanError,
};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tokio::runtime::Runtime as TokioRuntime;

/// State object to store the stream service and runtime
pub struct ZelanState {
    // Using an Arc<Mutex<>> to allow for mutability behind shared ownership
    pub service: Arc<Mutex<StreamService>>,
    // Keep runtime reference to ensure it stays alive for the application lifetime
    _runtime: Arc<TokioRuntime>,
}

// Implement Clone for ZelanState to allow cloning the service in command handlers
impl Clone for ZelanState {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            _runtime: self._runtime.clone(),
        }
    }
}

impl ZelanState {
    pub fn new() -> Result<Self> {
        // Create tokio runtime
        let rt = TokioRuntime::new()?;
        let rt_arc = Arc::new(rt);

        // Create service
        let service = StreamService::new();
        let service_arc = Arc::new(Mutex::new(service));

        Ok(Self {
            service: service_arc,
            _runtime: rt_arc,
        })
    }

    /// Initialize background services
    pub fn init_services(&self) -> Result<()> {
        let rt = self._runtime.clone();
        let service = self.service.clone();

        // We initialize services in a separate thread to avoid blocking the main thread
        std::thread::spawn(move || {
            rt.block_on(async {
                // Get service with exclusive access
                let mut service_guard = service.lock().unwrap();

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

                // Register the test adapter with settings
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
                service_guard.register_adapter(test_adapter, Some(test_settings)).await;

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
        });

        Ok(())
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde_json;
use tauri::State;

/// Get current status of the event bus
#[tauri::command]
pub async fn get_event_bus_status(
    state: State<'_, ZelanState>,
) -> Result<serde_json::Value, ZelanError> {
    // Use a scope to ensure the lock is released before .await
    let event_bus = match state.service.lock() {
        Ok(service) => service.event_bus().clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

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
    // Clone the service outside the .await
    let service_clone = match state.service.lock() {
        Ok(service_guard) => service_guard.clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

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
    // Clone the service outside the .await
    let service_clone = match state.service.lock() {
        Ok(service_guard) => service_guard.clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

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
    adapter_name: String,
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

    // Clone the service outside the .await
    let service_clone = match state.service.lock() {
        Ok(service_guard) => service_guard.clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

    // Update the settings
    service_clone
        .update_adapter_settings(&adapter_name, adapter_settings)
        .await?;

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

    // Get the event bus from the service
    let event_bus = match state.service.lock() {
        Ok(service) => service.event_bus().clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

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
pub fn get_websocket_info(state: State<'_, ZelanState>) -> Result<serde_json::Value, ZelanError> {
    // Get the WebSocket configuration
    let config = match state.service.lock() {
        Ok(service) => service.ws_config().clone(),
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

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
pub fn set_websocket_port(port: u16, state: State<'_, ZelanState>) -> Result<String, ZelanError> {
    // Validate port number
    if port < 1024 {
        return Err(ZelanError {
            code: ErrorCode::ConfigInvalid,
            message: "Invalid port number".to_string(),
            context: Some(format!("Port must be between 1024 and 65535, got {}", port)),
            severity: ErrorSeverity::Error,
        });
    }

    // Get exclusive access to the service
    let mut service_guard = match state.service.lock() {
        Ok(guard) => guard,
        Err(e) => {
            return Err(ZelanError {
                code: ErrorCode::Internal,
                message: "Failed to acquire service lock".to_string(),
                context: Some(e.to_string()),
                severity: ErrorSeverity::Error,
            });
        }
    };

    // Update WebSocket configuration
    let mut config = service_guard.ws_config().clone();
    config.port = port;
    service_guard.set_ws_config(config);

    // Note: This doesn't restart the server, you'll need to restart the app
    // for the port change to take effect

    Ok(format!(
        "WebSocket port updated to {}. Restart the application for changes to take effect.",
        port
    ))
}

/// Initialize the Zelan plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("zelan")
        .setup(|app, _api| {
            // Create and initialize the zelan state
            let zelan_state = ZelanState::new()?;

            // Initialize services
            zelan_state.init_services()?;

            // Store state for later use
            app.manage(zelan_state);

            Ok(())
        })
        .build()
}
