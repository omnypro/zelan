// This file is now deprecated - import from @shared/types instead
import { 
  BaseEventSchema, 
  BaseEvent, 
  EventType 
} from '@shared/types';
import { z } from 'zod';
import { v4 as uuidv4 } from 'uuid';

// Re-export for backward compatibility
export { 
  BaseEventSchema, 
  BaseEvent, 
  EventType 
};

/**
 * Creates a typed, validated event
 */
export function createEvent<T extends z.ZodType>(
  schema: T,
  data: Partial<BaseEvent> & { type: string; source: string }
): z.infer<T> & BaseEvent {
  const event = {
    id: uuidv4(), // Using uuid instead of random string 
    timestamp: Date.now(),
    ...data,
  };
  
  return schema.parse(event) as z.infer<T> & BaseEvent;
}