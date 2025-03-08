import { BaseEvent as IBaseEvent, EventCategory } from '@s/types/events'

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
  source = 'system'
): IBaseEvent<T> {
  return {
    id: crypto.randomUUID(),
    timestamp: Date.now(),
    source,
    category,
    type,
    payload
  }
}
