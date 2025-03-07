// Export createEvent utility without re-exporting types
import { BaseEvent } from '@shared/types';
import { z } from 'zod';
import { v4 as uuidv4 } from 'uuid';

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