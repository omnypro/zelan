import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// Import TypeScript backend modules
import { initBackend } from './backend';

// Initialize TypeScript backend before rendering the UI
async function initializeApp() {
  try {
    console.log('Initializing TypeScript backend...');
    await initBackend();
    console.log('TypeScript backend initialized successfully');
    
    // Expose backend API to window for Tauri bridge
    window.__ZELAN_TS_BACKEND = {
      // Main initialization function
      initBackend: async () => {
        console.log('Initializing TypeScript backend from Tauri bridge');
        const result = await initBackend();
        return result;
      },
      
      // Test adapter functions
      connectTestAdapter: async () => {
        console.log('Connecting Test adapter from Tauri bridge');
        // Import dynamically to reduce initial load time
        const { getTestAdapter } = await import('./backend/adapters/test-adapter');
        const adapter = getTestAdapter();
        await adapter.connect();
        return { status: 'connected', adapter: adapter.getName() };
      },
      
      disconnectTestAdapter: async () => {
        console.log('Disconnecting Test adapter from Tauri bridge');
        // Import dynamically to reduce initial load time
        const { getTestAdapter } = await import('./backend/adapters/test-adapter');
        const adapter = getTestAdapter();
        await adapter.disconnect();
        return { status: 'disconnected', adapter: adapter.getName() };
      },
      
      configureTestAdapter: async (config: any) => {
        console.log('Configuring Test adapter from Tauri bridge', config);
        // Import dynamically to reduce initial load time
        const { getTestAdapter } = await import('./backend/adapters/test-adapter');
        const adapter = getTestAdapter();
        await adapter.configure(config);
        return { 
          status: 'configured', 
          adapter: adapter.getName(),
          config: config
        };
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
