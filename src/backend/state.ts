/**
 * Central state management for Zelan using Zustand
 */

import { create } from 'zustand';
import { EventBus } from './event-bus';
import { ServiceStatus, AdapterSettings } from './types';
import { getEventBus } from './index';

/**
 * State interface for the whole application
 */
interface ZelanState {
  // Event bus and stats
  eventBus: EventBus;
  eventBusStats: {
    events_published: number;
    events_dropped: number;
    source_counts: Record<string, number>;
    type_counts: Record<string, number>;
  };
  
  // Adapters state
  adapterStatuses: Record<string, ServiceStatus>;
  adapterSettings: Record<string, AdapterSettings>;
  
  // WebSocket configuration
  wsConfig: {
    port: number;
  };

  // Actions to modify state
  updateAdapterStatus: (name: string, status: ServiceStatus) => void;
  updateAdapterSettings: (name: string, settings: AdapterSettings) => void;
  updateWsConfig: (port: number) => void;
  updateEventBusStats: () => Promise<void>;
  registerAdapter: (name: string, settings: AdapterSettings) => void;
}

/**
 * Create the Zustand store
 */
export const useZelanStore = create<ZelanState>((set, get) => ({
  // Initial state
  eventBus: getEventBus(),
  eventBusStats: {
    events_published: 0,
    events_dropped: 0,
    source_counts: {},
    type_counts: {},
  },
  adapterStatuses: {},
  adapterSettings: {},
  wsConfig: {
    port: 9000
  },
  
  // Actions
  updateAdapterStatus: (name, status) => set(state => ({
    adapterStatuses: {
      ...state.adapterStatuses,
      [name]: status
    }
  })),
  
  updateAdapterSettings: (name, settings) => set(state => ({
    adapterSettings: {
      ...state.adapterSettings,
      [name]: settings
    }
  })),
  
  updateWsConfig: (port) => set({
    wsConfig: { port }
  }),
  
  updateEventBusStats: async () => {
    const { eventBus } = get();
    const stats = await eventBus.getStats();
    set({ eventBusStats: stats });
  },
  
  registerAdapter: (name, settings) => set(state => ({
    adapterSettings: {
      ...state.adapterSettings,
      [name]: settings
    },
    adapterStatuses: {
      ...state.adapterStatuses,
      [name]: settings.enabled ? ServiceStatus.Disconnected : ServiceStatus.Disabled
    }
  })),
}));

/**
 * Subscribe to event bus for updates
 */
export function initializeStateSubscriptions() {
  const store = useZelanStore.getState();
  const { eventBus } = store;
  
  // Set up a timer to update event bus stats periodically
  setInterval(() => {
    store.updateEventBusStats();
  }, 2000);
  
  // Subscribe to events from the event bus
  const unsubscribe = eventBus.subscribe(event => {
    // When adapter status changes, update the store
    if (event.event_type === 'adapter.status_changed') {
      const { adapter, status } = event.payload;
      if (adapter && status) {
        store.updateAdapterStatus(adapter, status);
      }
    }
    
    // Update stats on any event
    store.updateEventBusStats();
  });
  
  // Return unsubscribe function for cleanup
  return unsubscribe;
}

/**
 * Create adapter settings with defaults
 */
export function createAdapterSettings(
  name: string, 
  enabled: boolean = false,
  config: any = {},
  displayName?: string,
  description?: string
): AdapterSettings {
  return {
    enabled,
    config,
    display_name: displayName || name,
    description: description || `${name} adapter`
  };
}

/**
 * Helper to watch for adapter status changes
 */
export function watchAdapterStatus(name: string, callback: (status: ServiceStatus) => void) {
  const unsubscribe = useZelanStore.subscribe((state) => {
    const status = state.adapterStatuses[name];
    callback(status);
  });
  return unsubscribe;
}
