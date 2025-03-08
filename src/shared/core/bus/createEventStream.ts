import { Observable } from 'rxjs'
import { EventBus } from './EventBus'
import { BaseEvent, EventCategory } from '@s/types/events'
import { EventFilterCriteria } from '@s/utils/filters/event-filter'

/**
 * Event stream type
 */
export type EventStream<T> = Observable<BaseEvent<T>>

/**
 * Create an event stream filtered by category and optionally by type
 * @param eventBus The event bus instance
 * @param category Event category to filter by
 * @param type Optional event type to filter by
 * @returns Observable of filtered events
 */
export function createEventStream<T>(
  eventBus: EventBus,
  category: EventCategory,
  type?: string
): EventStream<T> {
  if (type) {
    return eventBus.getEventsByCategoryAndType<T>(category, type)
  } else {
    return eventBus.getEventsByCategory<T>(category)
  }
}

/**
 * Create an event stream with specified filter criteria
 * @param eventBus The event bus instance
 * @param criteria Filter criteria to apply
 * @returns Observable of filtered events
 */
export function createFilteredEventStream<T>(
  eventBus: EventBus,
  criteria: EventFilterCriteria<T>
): EventStream<T> {
  return eventBus.getFilteredEvents$<T>(criteria)
}