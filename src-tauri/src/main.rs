// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tracing::{debug, error, info};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use zelan_lib::{plugin, Config, StreamService};

fn main() {
    // Initialize the tracing subscriber for structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to info level if RUST_LOG is not set
            if cfg!(debug_assertions) {
                "zelan_lib=debug,warn".into()
            } else {
                "zelan_lib=info,warn".into()
            }
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Zelan application starting");

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

            // Create the Zelan state
            let zelan_state = plugin::init_state();

            // Create/get the store using the simpler approach
            let store = app.store("zelan.config.json");

            if let Ok(store) = &store {
                debug!("Store successfully opened");
                // Check if we have a configuration
                if store.has("config".to_string()) {
                    info!("Found existing configuration in store");
                    // Get the config value
                    if let Some(config_value) = store.get("config".to_string()) {
                        // Deserialize the config
                        debug!(config = ?config_value, "Config value from store");
                        match serde_json::from_value::<Config>(config_value.clone()) {
                            Ok(config) => {
                                info!("Successfully deserialized config, creating service");
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
                            Err(e) => {
                                error!(%e, "Failed to deserialize config");
                            }
                        }
                    }
                } else {
                    info!("No existing configuration found, creating default");

                    // Create a default configuration
                    let default_config = Config {
                        websocket: zelan_lib::WebSocketConfig { port: 9000 },
                        adapters: std::collections::HashMap::new(),
                    };

                    // Save the default config
                    store.set("config".to_string(), json!(default_config));
                    if let Err(e) = store.save() {
                        error!(%e, "Failed to save initial config");
                    } else {
                        info!("Default configuration saved to store");
                    }
                }
            } else {
                error!("Failed to access configuration store");
            }

            // Store as state for other components to access
            app.manage(store);

            // Initialize services properly using Tauri's async runtime
            info!("Starting service initialization");
            let zelan_state_clone = zelan_state.clone();
            tauri::async_runtime::spawn(async move {
                match zelan_state_clone.init_services().await {
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
            plugin::get_event_bus_status,
            plugin::get_adapter_statuses,
            plugin::get_adapter_settings,
            plugin::update_adapter_settings,
            plugin::send_test_event,
            plugin::get_websocket_info,
            plugin::set_websocket_port
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
