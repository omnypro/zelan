import { BaseEvent as IBaseEvent, EventCategory, EventSource } from '@s/types/events'

/**
 * Create a new event with the given parameters
 * @param category Event category
 * @param type Event type
 * @param payload Event payload
 * @param source Event source
 * @returns A new event object
 */
export function createEvent<T>(
  category: EventCategory,
  type: string,
  payload: T,
  source: EventSource | string = 'system'
): IBaseEvent<T> {
  // Create standardized source object
  const eventSource: EventSource =
    typeof source === 'string' ? { id: 'system', name: 'System', type: 'system' } : source

  return {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    source: eventSource,
    category,
    type,
    payload,
    data: payload, // For compatibility with API spec
    metadata: {
      version: '1.0'
    }
  }
}
