import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// Import TypeScript backend modules
import { initBackend, setupTauriCommands } from './backend';

// Initialize TypeScript backend before rendering the UI
async function initializeApp() {
  try {
    console.log('Initializing TypeScript backend...');
    const result = await initBackend();
    console.log('TypeScript backend initialized successfully', result);
    
    // Get Tauri bridge commands
    const commands = setupTauriCommands();
    
    // Expose backend API to window for Tauri bridge
    window.__ZELAN_TS_BACKEND = {
      // Main initialization function
      initBackend: async () => {
        console.log('Initializing TypeScript backend from Tauri bridge');
        return result;
      },
      
      // Test Adapter management functions
      connectTestAdapter: async () => {
        console.log('Connecting Test adapter from Tauri bridge');
        return commands.connectTestAdapter();
      },
      
      disconnectTestAdapter: async () => {
        console.log('Disconnecting Test adapter from Tauri bridge');
        return commands.disconnectTestAdapter();
      },
      
      configureTestAdapter: async (config: any) => {
        console.log('Configuring Test adapter from Tauri bridge', config);
        return commands.configureTestAdapter(config);
      },
      
      // OBS Adapter management functions
      connectOBSAdapter: async () => {
        console.log('Connecting OBS adapter from Tauri bridge');
        return commands.connectOBSAdapter();
      },
      
      disconnectOBSAdapter: async () => {
        console.log('Disconnecting OBS adapter from Tauri bridge');
        return commands.disconnectOBSAdapter();
      },
      
      configureOBSAdapter: async (config: any) => {
        console.log('Configuring OBS adapter from Tauri bridge', config);
        return commands.configureOBSAdapter(config);
      },
      
      // Status and statistics functions
      getEventBusStats: async () => {
        return commands.getEventBusStats();
      },
      
      getAdapterStatuses: () => {
        return commands.getAdapterStatuses();
      },
      
      getAdapterSettings: () => {
        return commands.getAdapterSettings();
      },
      
      updateAdapterSettings: async (adapterName: string, settings: any) => {
        console.log('Updating adapter settings from Tauri bridge', adapterName, settings);
        return commands.updateAdapterSettings(adapterName, settings);
      }
    };
  } catch (error) {
    console.error('Failed to initialize TypeScript backend:', error);
  }
  
  // Now render the application
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
  
  // Signal to Tauri that frontend is loaded
  if (window.__TAURI__) {
    window.__TAURI__.event.emit('frontend-loaded', { 
      status: 'ready',
      ts_backend: !!window.__ZELAN_TS_BACKEND
    });
  }
}

// Define types for the window object
declare global {
  interface Window {
    __ZELAN_TS_BACKEND?: any;
    __TAURI__?: any;
  }
}

// Start the initialization process
initializeApp();
