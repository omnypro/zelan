import { z } from 'zod';

/**
 * Base event schema that all events must follow
 */
export const BaseEventSchema = z.object({
  id: z.string(),
  timestamp: z.number(),
  type: z.string(),
  source: z.string(),
});

export type BaseEvent = z.infer<typeof BaseEventSchema>;

/**
 * Event types enumeration for type safety
 */
export enum EventType {
  // System events
  SYSTEM_STARTUP = 'system.startup',
  SYSTEM_SHUTDOWN = 'system.shutdown',
  
  // Authentication events
  AUTH_STARTED = 'auth.started',
  AUTH_COMPLETED = 'auth.completed',
  AUTH_FAILED = 'auth.failed',
  AUTH_TOKEN_REFRESHED = 'auth.token.refreshed',
  AUTH_TOKEN_EXPIRED = 'auth.token.expired',
  
  // Adapter events
  ADAPTER_CONNECTED = 'adapter.connected',
  ADAPTER_DISCONNECTED = 'adapter.disconnected',
  ADAPTER_ERROR = 'adapter.error',
  
  // Service-specific events can be added later
}

/**
 * Creates a typed, validated event
 */
export function createEvent<T extends z.ZodType>(
  schema: T,
  data: Partial<BaseEvent> & { type: string; source: string }
): z.infer<T> & BaseEvent {
  const event = {
    id: Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15),
    timestamp: Date.now(),
    ...data,
  };
  
  return schema.parse(event) as z.infer<T> & BaseEvent;
}