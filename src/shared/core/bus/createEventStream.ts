import { Observable } from 'rxjs'
import { EventBus, EventFilterCriteria } from './EventBus'
import { BaseEvent, EventCategory } from '@s/types/events'

/**
 * Event stream type
 */
export type EventStream<T> = Observable<BaseEvent<T>>

/**
 * Create an event stream filtered by category and optionally by type
 */
export function createEventStream<T>(eventBus: EventBus, category: EventCategory, type?: string): EventStream<T> {
  if (type) {
    return eventBus.getEventsByCategoryAndType$<T>(category, type)
  } else {
    return eventBus.getEventsByCategory$<T>(category)
  }
}

/**
 * Create an event stream with specified filter criteria
 */
export function createFilteredEventStream<T>(eventBus: EventBus, criteria: EventFilterCriteria<T>): EventStream<T> {
  return eventBus.getFilteredEvents$<T>(criteria)
}