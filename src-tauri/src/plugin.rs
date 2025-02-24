use anyhow::Result;
use std::sync::{Arc, Mutex};
use tauri::{plugin::{Builder, TauriPlugin}, Runtime, Manager};
use tokio::runtime::Runtime as TokioRuntime;
use crate::{StreamService, adapters::TestAdapter};

/// State object to store the stream service and runtime
pub struct ZelanState {
    // Using an Arc<Mutex<>> to allow for mutability behind shared ownership
    pub service: Arc<Mutex<StreamService>>,
    // Keep runtime reference to ensure it stays alive for the application lifetime
    _runtime: Arc<TokioRuntime>,
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
                
                // Register the test adapter
                let test_adapter = TestAdapter::new(service_guard.event_bus());
                service_guard.register_adapter(test_adapter).await;
                
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
                            println!("Background listener received event: {}.{}", event.source(), event.event_type());
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

// Re-export the greet command for use in the plugin
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Initialize the Zelan plugin
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("zelan")
        .invoke_handler(tauri::generate_handler![greet])
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
