// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zelan_lib::plugin;

fn main() {
    // Build and run the Tauri application with our plugin
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(plugin::init())
        .setup(|_app| {
            println!("Tauri application starting up");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            plugin::greet, 
            plugin::get_event_bus_status, 
            plugin::get_adapter_statuses,
            plugin::send_test_event
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
