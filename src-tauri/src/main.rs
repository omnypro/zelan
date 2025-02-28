// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::Arc;
use tauri::{App, AppHandle, Manager, Runtime};
use tauri_plugin_store::{Store, StoreExt};
use tracing::{debug, error, info};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use zelan_lib::{plugin, Config, StreamService};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy,  NSVisualEffectMaterial};

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

fn setup_decorations(app: &mut App) {
    info!("Setting up window decorations");

    let window = app.get_webview_window("main").unwrap();

    #[cfg(target_os = "macos")]
    apply_vibrancy(&window, NSVisualEffectMaterial::WindowBackground, None, Some(8.0)).expect("Unsupported platform! 'apply vibrancy' is only supported on macOS");
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
                "zelan_lib=info,zelan_lib::event_bus=trace,zelan_lib::adapters::obs=debug,warn"
                    .into()
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
    debug!("To reduce noisy logs, use RUST_LOG=zelan_lib=info,zelan_lib::event_bus=trace");

    // Build and run the Tauri application with our plugin
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            info!("Setting up Tauri application");

            setup_decorations(app);

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
            plugin::connect_adapter,
            plugin::disconnect_adapter,
            plugin::get_adapter_auth_status,
            plugin::get_adapter_settings,
            plugin::get_adapter_statuses,
            plugin::get_event_bus_status,
            plugin::get_websocket_info,
            plugin::send_test_event,
            plugin::set_websocket_port,
            plugin::update_adapter_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
