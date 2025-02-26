/**
 * Backend TypeScript implementation for Zelan
 * Main entry point for the TypeScript business logic
 */

import { EventBus } from './event-bus';
import { ServiceStatus } from './types';
import { TestAdapter } from './adapters/test-adapter';
import { initializeStateSubscriptions, useZelanStore, createAdapterSettings, eventBusInstance } from './state';

// Export all backend modules
export * from './event-bus';
export * from './types';
export * from './adapters/base-adapter';
export * from './adapters/test-adapter';
export * from './adapters/obs-adapter';
export * from './state';

/**
 * Get the EventBus instance 
 */
export function getEventBus(): EventBus {
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
    // Using dynamic import for ESM compatibility
    import('./adapters/obs-adapter').then(module => {
      const adapter = new module.OBSAdapter(eventBusInstance);
      obsAdapter = adapter;
      registerAdapter(adapter);
    });
    // Return a placeholder until loaded
    return {
      getName: () => 'obs',
      isConnected: () => false,
      connect: async () => {
        console.warn('OBS adapter not fully loaded yet');
        return Promise.resolve();
      },
      disconnect: async () => Promise.resolve(),
      configure: async () => Promise.resolve()
    } as any;
  }
  return obsAdapter;
}

/**
 * Initialize the TypeScript backend
 */
export async function initBackend() {
  console.log('Initializing TypeScript backend');
  
  // Print event bus info to verify it exists
  console.log('Event bus instance:', eventBusInstance);
  
  // Initialize the state system
  const unsubscribe = initializeStateSubscriptions();
  
  // Reset the adapter registry to ensure clean state
  Object.keys(adapters).forEach(key => {
    delete adapters[key];
  });
  
  // Create and initialize adapters
  const testAdapter = getTestAdapter();
  console.log('Test adapter created:', testAdapter.getName());
  
  // Explicitly register the test adapter to ensure it's in the registry
  console.log('Explicitly registering test adapter');
  registerAdapter(testAdapter);
  
  // Try to initialize OBS adapter
  try {
    const obsAdapter = getOBSAdapter();
    console.log('OBS adapter created:', obsAdapter.getName());
  } catch (e) {
    console.warn('Failed to initialize OBS adapter:', e);
  }
  
  // Publish a test event to ensure the event bus is working
  try {
    await eventBusInstance.publish({
      id: `init-${Date.now()}`,
      source: 'system',
      event_type: 'backend.initialized',
      timestamp: new Date().toISOString(),
      payload: {
        message: 'TypeScript backend initialized'
      }
    });
    console.log('Published initialization event to event bus');
  } catch (err) {
    console.error('Failed to publish event to event bus:', err);
  }
  
  // Get the Zustand store
  const store = useZelanStore.getState();
  
  // Update event bus stats initially
  await store.updateEventBusStats();
  
  // Log adapters state to verify registration
  console.log('Registered adapters:', Object.keys(adapters));
  console.log('Adapter statuses:', store.adapterStatuses);
  
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
      
      console.log('Connecting test adapter');
      
      // Update adapter status in store
      store.updateAdapterStatus(adapter.getName(), ServiceStatus.Connecting);
      
      // Connect adapter
      await adapter.connect();
      
      // Update status after connection
      store.updateAdapterStatus(adapter.getName(), ServiceStatus.Connected);
      
      // Publish a test event to verify the event bus works
      await eventBusInstance.publish({
        id: `test-${Date.now()}`,
        source: 'test',
        event_type: 'adapter.connected',
        timestamp: new Date().toISOString(),
        payload: {
          adapter: 'test',
          message: 'Test adapter connected successfully'
        }
      });
      
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
      console.log('Getting event bus stats');
      
      try {
        // Try to publish a test event to ensure the event bus is working
        await eventBusInstance.publish({
          id: `stats-${Date.now()}`,
          source: 'test',
          event_type: 'stats.request',
          timestamp: new Date().toISOString(),
          payload: { message: 'Stats requested' }
        });
        console.log('Published stats.request event');
      } catch (err) {
        console.error('Failed to publish stats event:', err);
      }
      
      // Get stats directly from event bus
      const stats = await eventBusInstance.getStats();
      console.log('Raw event bus stats:', stats);
      
      // Update stats in store
      await store.updateEventBusStats();
      
      // Log store stats for comparison
      console.log('Store event bus stats:', store.eventBusStats);
      
      // Generate a default stats object in case the event bus stats are empty
      const defaultStats = {
        events_published: stats.events_published || 0,
        events_dropped: stats.events_dropped || 0,
        source_counts: stats.source_counts || {},
        type_counts: stats.type_counts || {}
      };
      
      // Return stats from store or the default stats
      return store.eventBusStats || defaultStats;
    },
    
    getAdapterStatuses: () => {
      // Log current adapter statuses for debugging
      console.log('Current adapter statuses:', store.adapterStatuses);
      console.log('Registered adapters:', Object.keys(adapters));
      
      // If no statuses are available but we have adapters registered, create default statuses
      if (Object.keys(store.adapterStatuses).length === 0 && Object.keys(adapters).length > 0) {
        console.log('No adapter statuses found in store but adapters exist, creating defaults');
        const defaultStatuses = {};
        
        // Create a default status for each registered adapter
        Object.keys(adapters).forEach(adapter_name => {
          const adapter = adapters[adapter_name];
          const isConnected = adapter.isConnected ? adapter.isConnected() : false;
          store.updateAdapterStatus(adapter_name, isConnected ? ServiceStatus.Connected : ServiceStatus.Disconnected);
          
          defaultStatuses[adapter_name] = isConnected ? ServiceStatus.Connected : ServiceStatus.Disconnected;
        });
        
        console.log('Created default statuses:', defaultStatuses);
        return defaultStatuses;
      }
      
      // Return statuses from store
      return store.adapterStatuses;
    },
    
    getAdapterSettings: () => {
      // Log current adapter settings for debugging
      console.log('Current adapter settings:', store.adapterSettings);
      console.log('Registered adapters:', Object.keys(adapters));
      
      // If no settings are available but we have adapters registered, create default settings
      if (Object.keys(store.adapterSettings).length === 0 && Object.keys(adapters).length > 0) {
        console.log('No adapter settings found in store but adapters exist, creating defaults');
        const defaultSettings = {};
        
        // Create default settings for each adapter
        Object.keys(adapters).forEach(adapter_name => {
          const settings = createAdapterSettings(
            adapter_name,
            true,
            {},
            adapter_name.charAt(0).toUpperCase() + adapter_name.slice(1),
            `${adapter_name.charAt(0).toUpperCase() + adapter_name.slice(1)} adapter`
          );
          
          // Register in store
          store.registerAdapter(adapter_name, settings);
          defaultSettings[adapter_name] = settings;
        });
        
        console.log('Created default settings:', defaultSettings);
        return defaultSettings;
      }
      
      // Return settings from store
      return store.adapterSettings;
    },
    
    updateAdapterSettings: async (adapter_name: string, settings: any) => {
      // Get the adapter
      let adapter;
      if (adapter_name === 'test') {
        adapter = getTestAdapter();
      } else if (adapter_name === 'obs') {
        adapter = getOBSAdapter();
      } else {
        return { 
          status: 'error', 
          message: `Unknown adapter: ${adapter_name}` 
        };
      }
      
      // Update settings in store
      store.updateAdapterSettings(adapter_name, settings);
      
      // Configure the adapter if it exists
      if (adapter && settings.config) {
        await adapter.configure(settings.config);
      }
      
      return { 
        status: 'success', 
        adapter: adapter_name,
        settings: settings
      };
    }
  };
}
