import { z } from 'zod';
import { v4 as uuidv4 } from 'uuid';

/**
 * Base event schema that all events must follow
 */
export const BaseEventSchema = z.object({
  id: z.string(),
  timestamp: z.number(),
  type: z.string(),
  source: z.string(),
  adapterId: z.string().optional(),
  data: z.record(z.string(), z.any()).optional(),
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
  
  // OBS specific events
  OBS_SCENE_CHANGED = 'obs.scene.changed',
  OBS_STREAMING_STATUS = 'obs.streaming.status',
  OBS_SOURCE_VISIBILITY = 'obs.source.visibility',
  OBS_CUSTOM_EVENT = 'obs.custom.event',
  
  // Test specific events
  TEST_EVENT = 'test.event',
}

/**
 * OBS event types enumeration
 */
export enum ObsEventType {
  SCENE_CHANGED = 'obs.scene.changed',
  STREAMING_STARTED = 'obs.streaming.started',
  STREAMING_STOPPED = 'obs.streaming.stopped',
  RECORDING_STARTED = 'obs.recording.started',
  RECORDING_STOPPED = 'obs.recording.stopped',
  SOURCE_ACTIVATED = 'obs.source.activated',
  SOURCE_DEACTIVATED = 'obs.source.deactivated',
  CONNECTION_STATUS = 'obs.connection.status',
}

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