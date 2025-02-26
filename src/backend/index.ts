/**
 * Backend TypeScript implementation for Zelan
 * Main entry point for the TypeScript business logic
 */

import { EventBus } from './event-bus';
import { ServiceStatus } from './types';
import { TestAdapter } from './adapters/test-adapter';

// Export all backend modules
export * from './event-bus';
export * from './types';
export * from './adapters/base-adapter';
export * from './adapters/test-adapter';

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

// Create a test adapter instance by default
let testAdapter: TestAdapter | null = null;

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
 * Initialize the TypeScript backend
 */
export async function initBackend() {
  console.log('Initializing TypeScript backend');
  const eventBus = getEventBus();
  
  // Create a test adapter instance
  getTestAdapter();
  
  // Log all modules available
  console.log('Available TypeScript backend modules:', {
    eventBus: !!eventBusInstance,
    testAdapter: !!testAdapter,
    adapters: Object.keys(adapters)
  });
  
  // Return initialization status
  return {
    status: 'initialized',
    timestamp: new Date().toISOString(),
    adapters: Object.keys(adapters),
    eventBus: {
      events_published: eventBus ? await eventBus.getStats().then(stats => stats.events_published) : 0
    }
  };
}

/**
 * Setup a bridge between TypeScript and Rust
 * This is the main entry point for Tauri commands to interact with TypeScript
 */
export function setupTauriCommands() {
  // This function will be called when Tauri needs to interact with TypeScript
  return {
    connectTestAdapter: async () => {
      const adapter = getTestAdapter();
      await adapter.connect();
      return { status: 'connected', adapter: adapter.getName() };
    },
    
    disconnectTestAdapter: async () => {
      const adapter = getTestAdapter();
      await adapter.disconnect();
      return { status: 'disconnected', adapter: adapter.getName() };
    },
    
    configureTestAdapter: async (config: any) => {
      const adapter = getTestAdapter();
      await adapter.configure(config);
      return { 
        status: 'configured', 
        adapter: adapter.getName(),
        config: config
      };
    },
    
    getEventBusStats: async () => {
      const eventBus = getEventBus();
      const stats = await eventBus.getStats();
      return stats;
    }
  };
}