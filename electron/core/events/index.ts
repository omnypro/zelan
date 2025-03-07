// Export core event components
export { EventBus } from './eventBus';
export { EventCache } from './eventCache';
export { EventStream } from './eventStream';
export { createEvent } from './types';

// Export types from shared
export { 
  EventType, 
  BaseEventSchema 
} from '@shared/types';
export type { 
  BaseEvent 
} from '@shared/types';