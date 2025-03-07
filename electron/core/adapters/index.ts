// Import and re-export adapter-related types from shared types
export { 
  AdapterState, 
  AdapterConfigSchema,
  ObsAdapterConfigSchema,
  TestAdapterConfigSchema 
} from '@shared/types';
export type { 
  AdapterConfig, 
  ServiceAdapter,
  ObsAdapterConfig,
  TestAdapterConfig
} from '@shared/types';

// Export adapter implementations
export { BaseAdapter } from './baseAdapter';
export { AdapterManager } from './adapterManager';
export { TestAdapter } from './testAdapter';
export { ObsAdapter } from './obsAdapter';