// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Listener;
use tauri::State;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::{Store, StoreExt};
use tracing::{debug, error, info};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use zelan_lib::{plugin, AdapterSettings, Config, ErrorCode, ErrorSeverity, StreamService, ZelanError, plugin::ZelanState};

/// Initialize all stores used by the application
fn initialize_stores<R: Runtime>(app: &AppHandle<R>) -> Result<(Arc<Store<R>>, Arc<Store<R>>)> {
    // Create/get the configuration store for general settings
    let config_store = app.store("zelan.config.json").map_err(|e| {
        error!("Failed to create config store: {}", e);
        anyhow!("Failed to create config store: {}", e)
    })?;

    // Create/get the secure store for sensitive data like tokens
    let secure_store = app.store("zelan.secure.json").map_err(|e| {
        error!("Failed to create secure store: {}", e);
        anyhow!("Failed to create secure store: {}", e)
    })?;

    // Initialize the secure store if needed
    if !secure_store.has("initialized") {
        info!("Initializing secure store");
        secure_store.set("initialized", json!(true));
        if let Err(e) = secure_store.save() {
            error!(%e, "Failed to initialize secure store");
            return Err(anyhow!("Failed to initialize secure store: {}", e));
        }
        info!("Secure store initialized");
    } else {
        debug!("Secure store already initialized");
    }

    // Initialize config store with default config if needed
    if !config_store.has("config") {
        info!("No existing configuration found, creating default");

        // Create a default configuration
        let default_config = Config {
            websocket: zelan_lib::WebSocketConfig { port: 9000 },
            adapters: std::collections::HashMap::new(),
        };

        // Save the default config
        config_store.set("config", json!(default_config));
        if let Err(e) = config_store.save() {
            error!(%e, "Failed to save initial config");
            return Err(anyhow!("Failed to save initial config: {}", e));
        }
        info!("Default configuration saved to store");
    } else {
        debug!("Configuration store already has config entry");
    }

    Ok((config_store, secure_store))
}

/// Load configuration from the config store
fn load_configuration<R: Runtime>(config_store: &Arc<Store<R>>) -> Result<Config> {
    // Check if we have a configuration
    if !config_store.has("config") {
        return Err(anyhow!("No configuration found in store"));
    }

    // Get the config value
    let config_value = config_store
        .get("config")
        .ok_or_else(|| anyhow!("Failed to get config from store"))?;

    // Deserialize the config
    debug!(config = ?config_value, "Config value from store");
    let config = serde_json::from_value::<Config>(config_value.clone()).map_err(|e| {
        error!(%e, "Failed to deserialize config");
        anyhow!("Failed to deserialize config: {}", e)
    })?;

    info!("Successfully deserialized config");
    Ok(config)
}

fn main() {
    // Load environment variables from .env file if it exists
    let env_file_path = match dotenvy::dotenv() {
        Ok(path) => Some(path),
        Err(_) => None,
    };

    // Initialize the tracing subscriber for structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to info level if RUST_LOG is not set
            if cfg!(debug_assertions) {
                // More verbose in debug mode
                "zelan_lib=debug,zelan_lib::adapters::obs=debug,warn".into()
            } else {
                // Less verbose in release mode
                "zelan_lib=info,zelan_lib::adapters::obs=info,warn".into()
            }
        }))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    info!("Zelan application starting");

    // Log environment loading after logger is initialized
    match env_file_path {
        Some(path) => info!("Loaded environment variables from {}", path.display()),
        None => debug!("No .env file found. Using existing environment variables."),
    };

    // Print log level configuration hint for developers
    debug!("Logging initialized. Set RUST_LOG environment variable to control log levels");
    debug!("Example: RUST_LOG=zelan_lib=trace,zelan_lib::adapters::obs=debug");
    debug!("Available modules: zelan_lib, zelan_lib::adapters::obs, zelan_lib::plugin");

    // Build and run the Tauri application with our plugin
    // A much simpler bridge using Tauri's recommended invoke pattern
    #[tauri::command]
    fn call_ts_backend(
        app: AppHandle,
        command: String,
        args: Option<serde_json::Value>,
    ) -> serde_json::Value {
        debug!(command = %command, "Delegating command to TypeScript backend");

        // For the legacy bridge, just return a placeholder response
        // This is a fallback while migrating to direct commands
        serde_json::json!({
            "status": "success",
            "command": command,
            "args": args,
            "message": "Using direct Rust commands now - this is a placeholder response. Please use the new direct commands instead."
        })
    }

    // No longer need response handlers since we're using direct commands
    
    // Direct commands for frontend to use - using proper Tauri 2.0 command style
    #[tauri::command]
    fn get_adapter_statuses(
        state: State<ZelanState>,
    ) -> Result<serde_json::Value, ZelanError> {
        // Clone the state to move into the async block
        let state_clone = state.inner().clone();
        
        // Execute the async code in a blocking manner
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            let statuses = service.get_all_statuses().await;
            
            // Convert to JSON
            serde_json::to_value(statuses).map_err(|e| {
                error!("Failed to serialize adapter statuses: {}", e);
                ZelanError {
                    code: ErrorCode::Internal,
                    message: "Failed to serialize adapter statuses".to_string(),
                    context: Some(e.to_string()),
                    severity: ErrorSeverity::Error,
                }
            })
        })
    }
    
    #[tauri::command]
    fn get_adapter_settings(
        state: State<ZelanState>,
    ) -> Result<serde_json::Value, ZelanError> {
        // Clone the state to move into the async block
        let state_clone = state.inner().clone();
        
        // Pull the data out of the state synchronously
        // This is a simpler approach for commands that don't need async processing
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            let settings = service.get_all_adapter_settings().await;
            
            // Convert to JSON
            serde_json::to_value(settings).map_err(|e| {
                error!("Failed to serialize adapter settings: {}", e);
                ZelanError {
                    code: ErrorCode::Internal,
                    message: "Failed to serialize adapter settings".to_string(),
                    context: Some(e.to_string()),
                    severity: ErrorSeverity::Error,
                }
            })
        })
    }
    
    #[tauri::command]
    fn get_event_bus_stats(
        state: State<ZelanState>,
    ) -> Result<serde_json::Value, ZelanError> {
        // Clone the state to move into the async block
        let state_clone = state.inner().clone();
        
        // Execute the async code in a blocking manner for the command
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            let event_bus = service.event_bus().clone();
            let stats = event_bus.get_stats().await;
            
            // Convert to JSON
            serde_json::to_value(stats).map_err(|e| {
                error!("Failed to serialize event bus stats: {}", e);
                ZelanError {
                    code: ErrorCode::Internal,
                    message: "Failed to serialize event bus stats".to_string(),
                    context: Some(e.to_string()),
                    severity: ErrorSeverity::Error,
                }
            })
        })
    }
    
    #[tauri::command]
    fn connect_adapter(
        state: State<ZelanState>,
        #[serde(rename = "adapterName")] adapter_name: String,
    ) -> Result<(), ZelanError> {
        // Clone the state and parameters for the async block
        let state_clone = state.inner().clone();
        let adapter_name_clone = adapter_name.clone();
        
        // Execute the async code in a blocking manner
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            service.connect_adapter(&adapter_name_clone).await
        })
    }
    
    #[tauri::command]
    fn disconnect_adapter(
        state: State<ZelanState>,
        #[serde(rename = "adapterName")] adapter_name: String,
    ) -> Result<(), ZelanError> {
        // Clone the state and parameters for the async block
        let state_clone = state.inner().clone();
        let adapter_name_clone = adapter_name.clone();
        
        // Execute the async code in a blocking manner
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            service.disconnect_adapter(&adapter_name_clone).await
        })
    }
    
    #[tauri::command]
    fn update_adapter_settings(
        state: State<ZelanState>,
        #[serde(rename = "adapterName")] adapter_name: String,
        settings: AdapterSettings,
    ) -> Result<(), ZelanError> {
        // Clone the state and parameters for the async block
        let state_clone = state.inner().clone();
        let adapter_name_clone = adapter_name.clone();
        let settings_clone = settings.clone();
        
        // Execute the async code in a blocking manner
        tauri::async_runtime::block_on(async move {
            let service = state_clone.service.lock().await;
            service.update_adapter_settings(&adapter_name_clone, settings_clone).await
        })
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            info!("Setting up Tauri application");

            // No longer need ts-response listener since we're using direct commands

            // Create the Zelan state
            let zelan_state = plugin::init_state();

            // Initialize stores
            let app_handle = app.handle();
            let (config_store, secure_store) =
                initialize_stores(&app_handle).expect("Failed to initialize stores");

            // Load configuration and apply to service
            if let Ok(config) = load_configuration(&config_store) {
                // Create a StreamService from config and store it
                let new_service = StreamService::from_config(config);

                // Clone the state before moving it into the async block
                let zelan_state_clone = zelan_state.clone();

                // Spawn a task to update the service
                tauri::async_runtime::spawn(async move {
                    let mut service = zelan_state_clone.service.lock().await;
                    *service = new_service;
                    info!("Loaded configuration from persistent storage");
                });
            }

            // Store both stores as state for other components to access
            app.manage(config_store);
            app.manage(secure_store);

            // Initialize services properly using Tauri's async runtime
            info!("Starting service initialization");
            let zelan_state_clone = zelan_state.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match zelan_state_clone.init_services(app_handle).await {
                    Ok(_) => info!("Successfully initialized services"),
                    Err(e) => error!(%e, "Failed to initialize services"),
                }
            });
            debug!("Service initialization task spawned");

            // Store the state for later use
            app.manage(zelan_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Direct commands
            get_adapter_statuses,
            get_adapter_settings,
            get_event_bus_stats,
            connect_adapter,
            disconnect_adapter,
            update_adapter_settings,
            // Legacy commands
            call_ts_backend,
            plugin::send_test_event,
            plugin::get_websocket_info,
            plugin::set_websocket_port,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
