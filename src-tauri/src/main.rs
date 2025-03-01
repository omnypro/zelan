// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{Manager, State, Window};
use tauri_plugin_store::StoreBuilder;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
use zelan_lib::{adapters::{AdapterSettings, AdapterStatus}, AppConfig, AppState, WebSocketInfo};

// Re-export the auth commands
pub use zelan_lib::auth::commands::{
    start_authentication,
    check_auth_flow_status,
    get_auth_state,
    get_auth_token,
    clear_auth_tokens,
    get_available_auth_providers,
};

/// Get information about the WebSocket server
#[tauri::command]
async fn get_websocket_info(state: State<'_, Arc<AppState>>) -> Result<WebSocketInfo, String> {
    Ok(state.get_websocket_info().await)
}

/// Set the WebSocket server port
#[tauri::command]
async fn set_websocket_port(
    port: u16,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .set_websocket_port(port)
        .await
        .map_err(|e| e.to_string())
}

/// Start the WebSocket server
#[tauri::command]
async fn start_websocket_server(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .start_websocket_server()
        .await
        .map_err(|e| e.to_string())
}

/// Stop the WebSocket server
#[tauri::command]
async fn stop_websocket_server(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .stop_websocket_server()
        .await
        .map_err(|e| e.to_string())
}

/// Connect an adapter
#[tauri::command]
async fn connect_adapter(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .adapter_manager
        .connect_adapter(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Disconnect an adapter
#[tauri::command]
async fn disconnect_adapter(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .adapter_manager
        .disconnect_adapter(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Get adapter settings
#[tauri::command]
async fn get_adapter_settings(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AdapterSettings, String> {
    state
        .adapter_manager
        .get_adapter_settings(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Update adapter settings
#[tauri::command]
async fn update_adapter_settings(
    name: String,
    settings: AdapterSettings,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state
        .adapter_manager
        .update_adapter_settings(&name, settings)
        .await
        .map_err(|e| e.to_string())
}

/// Get adapter status
#[tauri::command]
async fn get_adapter_status(
    name: String,
    state: State<'_, Arc<AppState>>,
) -> Result<AdapterStatus, String> {
    state
        .adapter_manager
        .get_adapter_status(&name)
        .await
        .map_err(|e| e.to_string())
}

/// Get all adapter statuses
#[tauri::command]
async fn get_adapter_statuses(
    state: State<'_, Arc<AppState>>,
) -> Result<HashMap<String, AdapterStatus>, String> {
    Ok(state.adapter_manager.get_all_statuses().await)
}

/// Send a test event
#[tauri::command]
async fn send_test_event(
    event_type: String,
    payload: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Create test event
    let event = zelan_lib::events::StreamEvent::new(
        "test",
        &event_type,
        payload,
    );
    
    // Publish through event bus
    state
        .event_bus
        .publish(event)
        .await
        .map_err(|e| e.to_string())
        .map(|_| ())
}

/// Get event bus statistics
#[tauri::command]
async fn get_event_bus_status(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let stats = state.event_bus.get_stats().await;
    
    let result = json!({
        "events_published": stats.events_published,
        "events_dropped": stats.events_dropped,
        "capacity": state.event_bus.capacity(),
        "type_counts": stats.type_counts,
    });
    
    Ok(result)
}

/// Initialize stores used by the application
fn initialize_stores(app: &tauri::AppHandle) -> Result<()> {
    // Create/get the configuration store
    let config_store = app.state::<Arc<tauri_plugin_store::Store<tauri::AppHandle>>>();
    
    // Initialize config store with default config if needed
    if !config_store.has("config")? {
        info!("No existing configuration found, creating default");
        
        // Create a default configuration
        let default_config = AppConfig::default();
        
        // Save the default config
        config_store.set("config", json!(default_config))?;
        config_store.save()?;
        
        info!("Default configuration saved to store");
    } else {
        debug!("Configuration store already has config entry");
    }
    
    Ok(())
}

/// Load configuration from storage
fn load_configuration(app: &tauri::AppHandle) -> Result<AppConfig> {
    // Get the store
    let config_store = app.state::<Arc<tauri_plugin_store::Store<tauri::AppHandle>>>();
    
    // Check if we have a configuration
    if !config_store.has("config")? {
        return Err(anyhow!("No configuration found in store"));
    }
    
    // Get the config value
    let config_value = config_store
        .get("config")
        .ok_or_else(|| anyhow!("Failed to get config from store"))?;
    
    // Deserialize the config
    let config = serde_json::from_value::<AppConfig>(config_value)?;
    
    Ok(config)
}

/// Set up window decorations
fn setup_decorations(app: &mut tauri::App) {
    info!("Setting up window decorations");
    
    // Get the main window
    if let Some(window) = app.get_webview_window("main") {
        // Apply vibrancy on macOS
        #[cfg(target_os = "macos")]
        {
            let _ = apply_vibrancy(
                &window,
                NSVisualEffectMaterial::WindowBackground,
                None,
                Some(8.0),
            );
        }
    }
}

fn main() {
    // Load environment variables from .env file if it exists
    let _ = dotenvy::dotenv();
    
    // Initialize the tracing subscriber for structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to info level if RUST_LOG is not set
            if cfg!(debug_assertions) {
                // More verbose in debug mode
                "zelan_lib=info,zelan_lib::event_bus=trace,warn".into()
            } else {
                // Less verbose in release mode
                "zelan_lib=info,warn".into()
            }
        }))
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
    
    info!("Zelan application starting");
    
    // Build and run the Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the main window when another instance tries to launch
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(StoreBuilder::default().build())
        .setup(|app| {
            info!("Setting up Tauri application");
            
            // Setup window decorations
            setup_decorations(app);
            
            // Initialize stores
            initialize_stores(&app.handle())?;
            
            // Load configuration
            let config = match load_configuration(&app.handle()) {
                Ok(config) => {
                    info!("Loaded configuration from store");
                    config
                }
                Err(e) => {
                    warn!(error = %e, "Failed to load configuration, using defaults");
                    AppConfig::default()
                }
            };
            
            // Create application state
            let app_handle_arc = Arc::new(app.handle());
            let app_state = Arc::new(AppState::new(Some(app_handle_arc.clone())));
            
            // Initialize the application
            tokio::spawn({
                let app_state = app_state.clone();
                let app_handle_arc = app_handle_arc.clone();
                async move {
                    if let Err(e) = app_state.initialize(Some(app_handle_arc.clone())).await {
                        error!(error = %e, "Failed to initialize application");
                    }
                    
                    // Add adapters
                    if let Err(e) = app_state.add_twitch_adapter().await {
                        error!(error = %e, "Failed to add Twitch adapter");
                    }
                    
                    // Apply saved configuration
                    // TODO: Apply adapter settings from config
                    
                    // Start WebSocket server
                    if let Err(e) = app_state.start_websocket_server().await {
                        error!(error = %e, "Failed to start WebSocket server");
                    }
                }
            });
            
            // Store application state
            app.manage(app_state);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // WebSocket commands
            get_websocket_info,
            set_websocket_port,
            start_websocket_server,
            stop_websocket_server,
            
            // Adapter commands
            connect_adapter,
            disconnect_adapter,
            get_adapter_settings,
            update_adapter_settings,
            get_adapter_status,
            get_adapter_statuses,
            
            // Event bus commands
            send_test_event,
            get_event_bus_status,
            
            // Auth commands
            start_authentication,
            check_auth_flow_status,
            get_auth_state,
            get_auth_token,
            clear_auth_tokens,
            get_available_auth_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}