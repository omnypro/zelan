import { Observable, Subject } from 'rxjs'
import { filter } from 'rxjs/operators'
import { BaseEvent, EventCategory } from '@s/types/events'

export interface EventFilterCriteria<T = unknown> {
  category?: EventCategory
  type?: string
  sourceId?: string
  sourceType?: string
  since?: number
  predicate?: (event: BaseEvent<T>) => boolean
}

/**
 * Filters an array of events based on the provided criteria
 */
export function filterEvents<T = unknown>(events: BaseEvent<T>[], criteria: EventFilterCriteria<T>): BaseEvent<T>[] {
  if (!criteria || Object.keys(criteria).length === 0) return events

  return events.filter((event) => {
    // Check category
    if (criteria.category !== undefined && event.category !== criteria.category) {
      return false
    }

    // Check type
    if (criteria.type !== undefined && event.type !== criteria.type) {
      return false
    }

    // Check sourceId
    if (criteria.sourceId !== undefined && event.source?.id !== criteria.sourceId) {
      return false
    }

    // Check sourceType
    if (criteria.sourceType !== undefined && event.source?.type !== criteria.sourceType) {
      return false
    }

    // Check timestamp
    if (criteria.since !== undefined && event.timestamp < criteria.since) {
      return false
    }

    // Apply custom predicate if provided
    if (criteria.predicate && !criteria.predicate(event)) {
      return false
    }

    return true
  })
}

export interface EventBus {
  readonly events$: Observable<BaseEvent>
  getFilteredEvents$<T = unknown>(criteria: EventFilterCriteria<T>): Observable<BaseEvent<T>>
  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>>
  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>>
  getEventsByCategoryAndType$<T = unknown>(category: EventCategory, type: string): Observable<BaseEvent<T>>
  publish(event: BaseEvent): void
}

export class BaseEventBus implements EventBus {
  private eventsSubject = new Subject<BaseEvent>()
  readonly events$ = this.eventsSubject.asObservable()

  getFilteredEvents$<T = unknown>(criteria: EventFilterCriteria<T>): Observable<BaseEvent<T>> {
    // Simple cast - we're assuming T matches the event data type
    const stream$ = this.events$ as Observable<BaseEvent<T>>

    return stream$.pipe(
      filter((event) => {
        // Skip filtering if no criteria provided
        if (!criteria || Object.keys(criteria).length === 0) return true

        // Check category
        if (criteria.category !== undefined && event.category !== criteria.category) {
          return false
        }

        // Check type
        if (criteria.type !== undefined && event.type !== criteria.type) {
          return false
        }

        // Check sourceId
        if (criteria.sourceId !== undefined && event.source?.id !== criteria.sourceId) {
          return false
        }

        // Check sourceType
        if (criteria.sourceType !== undefined && event.source?.type !== criteria.sourceType) {
          return false
        }

        // Check timestamp
        if (criteria.since !== undefined && event.timestamp < criteria.since) {
          return false
        }

        // Apply custom predicate if provided
        if (criteria.predicate && !criteria.predicate(event as BaseEvent<T>)) {
          return false
        }

        return true
      })
    )
  }

  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category })
  }

  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ type })
  }

  getEventsByCategoryAndType$<T = unknown>(category: EventCategory, type: string): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category, type })
  }

  publish(event: BaseEvent): void {
    this.eventsSubject.next(event)
  }
}
