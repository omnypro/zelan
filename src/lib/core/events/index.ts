// Import event types from shared
export type {
  BaseEvent
} from '@shared/types';

// Import event types, enums, schemas, and utilities from shared types
export { 
  EventType,
  BaseEventSchema,
  createEvent
} from '@shared/types';

// Still need the event stream component from renderer
export * from './eventStream';

// Re-export for compatibility with existing code
export { EventBus } from '~/core/events/eventBus';
export { EventCache } from '~/core/events/eventCache';
