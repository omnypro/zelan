// Import adapter types from shared types
export type { 
  AdapterConfig, 
  AdapterState, 
  ServiceAdapter 
} from '@shared/types';

// Export base adapter and adapter manager
export { BaseAdapter } from './baseAdapter';
export { AdapterManager } from './adapterManager';

// Re-export adapter events and other types
export type { 
  AdapterEvent
} from './baseAdapter';

// Types for OBS adapter
export type { ObsAdapterConfig } from '@shared/types';

// Types for Test adapter
export type { TestAdapterConfig } from '@shared/types';

// Re-export event types
export { EventType } from '@shared/types';