// Re-export event types from electron implementation
export type {
  BaseEvent
} from '~/core/events/types';

// Re-export event enums and utils from electron implementation
export { 
  EventType,
  createEvent
} from '~/core/events/types';

// Re-export schemas from electron implementation
export {
  BaseEventSchema
} from '~/core/events/types';

// Still need the event stream component from renderer
export * from './eventStream';

// Re-export for compatibility with existing code
export { EventBus } from '~/core/events/eventBus';
export { EventCache } from '~/core/events/eventCache';
