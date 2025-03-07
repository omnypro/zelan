// Import adapter types from shared types
export type { 
  AdapterConfig, 
  AdapterState, 
  ServiceAdapter 
} from '@shared/types';

// Re-export adapter events and other types
export type { 
  AdapterEvent
} from '~/core/adapters/baseAdapter';

// Types for OBS adapter
export type {
  ObsAdapterConfig,
  ObsSceneChangedEvent,
  ObsStreamingStatusEvent,
  ObsSourceVisibilityEvent
} from '~/core/adapters/obsAdapter';

// Types for Test adapter
export type {
  TestAdapterConfig,
  TestEvent
} from '~/core/adapters/testAdapter';

// Re-export enums
export { ObsEventType } from '~/core/adapters/obsAdapter';