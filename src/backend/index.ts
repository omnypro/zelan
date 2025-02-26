/**
 * Backend TypeScript implementation for Zelan
 * Main entry point for the TypeScript business logic
 */

import { EventBus } from './event-bus';
import { ServiceStatus } from './types';
import { TestAdapter } from './adapters/test-adapter';
import { initializeStateSubscriptions, useZelanStore, createAdapterSettings } from './state';

// Export all backend modules
export * from './event-bus';
export * from './types';
export * from './adapters/base-adapter';
export * from './adapters/test-adapter';
export * from './adapters/obs-adapter';
export * from './state';

// Singleton instance of the EventBus
let eventBusInstance: EventBus | null = null;

/**
 * Get or create the EventBus instance
 */
export function getEventBus(): EventBus {
  if (!eventBusInstance) {
    eventBusInstance = new EventBus();
  }
  return eventBusInstance;
}

// Adapter registry to track active adapters
const adapters: Record<string, any> = {};

/**
 * Register a new adapter instance
 */
export function registerAdapter(adapter: any) {
  const name = adapter.getName();
  adapters[name] = adapter;
  
  // Register in state system
  const store = useZelanStore.getState();
  const settings = createAdapterSettings(
    name,
    true,
    {},
    name.charAt(0).toUpperCase() + name.slice(1),
    `${name.charAt(0).toUpperCase() + name.slice(1)} adapter`
  );
  store.registerAdapter(name, settings);
  
  return { name, status: 'registered' };
}

/**
 * Get all registered adapters
 */
export function getAdapters() {
  return adapters;
}

/**
 * Initialize the test adapter
 */
export function initTestAdapter(config?: any) {
  const eventBus = getEventBus();
  const adapter = new TestAdapter(eventBus, config);
  registerAdapter(adapter);
  return adapter;
}

// Create adapter instances (lazy loaded)
let testAdapter: TestAdapter | null = null;
let obsAdapter: import('./adapters/obs-adapter').OBSAdapter | null = null;

/**
 * Get the singleton test adapter instance
 */
export function getTestAdapter(): TestAdapter {
  if (!testAdapter) {
    testAdapter = initTestAdapter();
  }
  return testAdapter;
}

/**
 * Initialize the OBS adapter
 */
export function initOBSAdapter(config?: any) {
  const eventBus = getEventBus();
  // Import here to avoid circular dependencies
  const { OBSAdapter } = require('./adapters/obs-adapter');
  const adapter = new OBSAdapter(eventBus, config);
  registerAdapter(adapter);
  return adapter;
}

/**
 * Get the singleton OBS adapter instance
 */
export function getOBSAdapter(): import('./adapters/obs-adapter').OBSAdapter {
  if (!obsAdapter) {
    // Import dynamically to avoid circular dependencies
    const { OBSAdapter } = require('./adapters/obs-adapter');
    obsAdapter = initOBSAdapter();
  }
  return obsAdapter;
}

/**
 * Initialize the TypeScript backend
 */
export async function initBackend() {
  console.log('Initializing TypeScript backend');
  const eventBus = getEventBus();
  
  // Initialize the state system
  const unsubscribe = initializeStateSubscriptions();
  
  // Create adapter instances
  getTestAdapter();
  getOBSAdapter();
  
  // Get the Zustand store
  const store = useZelanStore.getState();
  
  // Update event bus stats initially
  await store.updateEventBusStats();
  
  // Log all modules available
  console.log('Available TypeScript backend modules:', {
    eventBus: !!eventBusInstance,
    testAdapter: !!testAdapter,
    adapters: Object.keys(adapters),
    store: Object.keys(store)
  });
  
  // Return initialization status
  return {
    status: 'initialized',
    timestamp: new Date().toISOString(),
    adapters: Object.keys(adapters),
    eventBus: {
      events_published: store.eventBusStats.events_published
    }
  };
}

/**
 * Setup a bridge between TypeScript and Rust
 * This is the main entry point for Tauri commands to interact with TypeScript
 */
export function setupTauriCommands() {
  // Get the store for state access
  const store = useZelanStore.getState();
  
  // This function will be called when Tauri needs to interact with TypeScript
  return {
    // Test Adapter Commands
    connectTestAdapter: async () => {
      const adapter = getTestAdapter();
      
      // Update adapter status in store
      store.updateAdapterStatus(adapter.getName(), ServiceStatus.Connecting);
      
      // Connect adapter
      await adapter.connect();
      
      // Update status after connection
      store.updateAdapterStatus(adapter.getName(), ServiceStatus.Connected);
      
      return { 
        status: 'connected', 
        adapter: adapter.getName() 
      };
    },
    
    disconnectTestAdapter: async () => {
      const adapter = getTestAdapter();
      
      // Update status before disconnection
      store.updateAdapterStatus(adapter.getName(), ServiceStatus.Disconnected);
      
      // Disconnect adapter
      await adapter.disconnect();
      
      return { 
        status: 'disconnected', 
        adapter: adapter.getName() 
      };
    },
    
    configureTestAdapter: async (config: any) => {
      const adapter = getTestAdapter();
      const name = adapter.getName();
      
      // Get current settings from store
      const currentSettings = store.adapterSettings[name] || createAdapterSettings(name);
      
      // Create updated settings
      const newSettings = {
        ...currentSettings,
        config: {
          ...currentSettings.config,
          ...config
        }
      };
      
      // Update settings in store
      store.updateAdapterSettings(name, newSettings);
      
      // Configure the adapter
      await adapter.configure(config);
      
      return { 
        status: 'configured', 
        adapter: name,
        config: config,
        settings: newSettings
      };
    },
    
    // OBS Adapter Commands
    connectOBSAdapter: async () => {
      const adapter = getOBSAdapter();
      
      // Connect adapter (status updates handled internally)
      await adapter.connect();
      
      return { 
        status: 'connected', 
        adapter: adapter.getName() 
      };
    },
    
    disconnectOBSAdapter: async () => {
      const adapter = getOBSAdapter();
      
      // Disconnect adapter (status updates handled internally)
      await adapter.disconnect();
      
      return { 
        status: 'disconnected', 
        adapter: adapter.getName() 
      };
    },
    
    configureOBSAdapter: async (config: any) => {
      const adapter = getOBSAdapter();
      
      // Configure the adapter (status updates handled internally)
      await adapter.configure(config);
      
      return { 
        status: 'configured', 
        adapter: adapter.getName(),
        config: config
      };
    },
    
    // General Commands
    getEventBusStats: async () => {
      // Update stats in store
      await store.updateEventBusStats();
      
      // Return stats from store
      return store.eventBusStats;
    },
    
    getAdapterStatuses: () => {
      // Return statuses from store
      return store.adapterStatuses;
    },
    
    getAdapterSettings: () => {
      // Return settings from store
      return store.adapterSettings;
    },
    
    updateAdapterSettings: async (adapterName: string, settings: any) => {
      // Get the adapter
      let adapter;
      if (adapterName === 'test') {
        adapter = getTestAdapter();
      } else if (adapterName === 'obs') {
        adapter = getOBSAdapter();
      } else {
        return { 
          status: 'error', 
          message: `Unknown adapter: ${adapterName}` 
        };
      }
      
      // Update settings in store
      store.updateAdapterSettings(adapterName, settings);
      
      // Configure the adapter if it exists
      if (adapter && settings.config) {
        await adapter.configure(settings.config);
      }
      
      return { 
        status: 'success', 
        adapter: adapterName,
        settings: settings
      };
    }
  };
}