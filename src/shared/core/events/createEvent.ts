import { BaseEvent, EventCategory, SystemEventType, SystemInfoPayload, ObsEventType } from '@s/types/events'

/**
 * Create a properly formatted event with consistent metadata
 */
export function createEvent<T>(
  category: EventCategory,
  type: string,
  data: T,
  sourceId: string,
  sourceName: string = sourceId,
  sourceType: string = category
): BaseEvent<T> {
  return {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    category,
    type,
    source: {
      id: sourceId,
      name: sourceName,
      type: sourceType
    },
    data,
    metadata: {
      version: '1.0'
    }
  }
}

/**
 * Create an OBS event with specific metadata
 */
export function createObsEvent<T>(type: ObsEventType, data: T, adapterId: string, adapterName: string): BaseEvent<T> {
  return createEvent(EventCategory.OBS, type, data, adapterId, adapterName, 'obs')
}

/**
 * Create a system event with specific metadata
 */
export function createSystemEvent(
  type: SystemEventType,
  message: string,
  level: 'info' | 'warning' | 'error' = 'info',
  details?: Record<string, unknown>
): BaseEvent<SystemInfoPayload> {
  return createEvent(
    EventCategory.SYSTEM,
    type,
    {
      message,
      level,
      details
    },
    'system',
    'System',
    'system'
  )
}

// Re-export for convenience
export { ObsEventType }
