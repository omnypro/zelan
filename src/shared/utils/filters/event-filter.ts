import { Observable } from 'rxjs'
import { filter } from 'rxjs/operators'
import { EventCategory, BaseEvent } from '../../types/events'

/**
 * Type for event filter criteria, supporting filtering by various event properties.
 */
export interface EventFilterCriteria<T = unknown> {
  /** Filter by event category */
  category?: EventCategory
  /** Filter by event type */
  type?: string
  /** Filter by source ID */
  sourceId?: string
  /** Filter by source type */
  sourceType?: string
  /** Filter by minimum timestamp (events after this time) */
  since?: number
  /** Custom filter predicate for advanced filtering */
  predicate?: (event: BaseEvent<T>) => boolean
}

/**
 * Applies filter criteria to produce a filter predicate function
 * @param criteria Filter criteria to apply
 * @returns A filter predicate function
 */
export function createEventFilterPredicate<T = unknown>(
  criteria: EventFilterCriteria<T>
): (event: BaseEvent<T>) => boolean {
  return (event: BaseEvent<T>) => {
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
    if (criteria.predicate && !criteria.predicate(event)) {
      return false
    }

    return true
  }
}

/**
 * Filters an array of events based on the provided criteria
 * @param events Array of events to filter
 * @param criteria Filter criteria to apply
 * @returns Filtered array of events
 */
export function filterEvents<T = unknown>(
  events: BaseEvent<T>[],
  criteria?: EventFilterCriteria<T>
): BaseEvent<T>[] {
  if (!criteria || Object.keys(criteria).length === 0) return events

  const predicate = createEventFilterPredicate(criteria)
  return events.filter(predicate)
}

/**
 * Creates an RxJS operator to filter an Observable of events
 * @param criteria Filter criteria to apply
 * @returns RxJS operator function
 */
export function filterEventStream<T = unknown>(criteria?: EventFilterCriteria<T>) {
  if (!criteria || Object.keys(criteria).length === 0) {
    return <E extends BaseEvent<T>>(source$: Observable<E>) => source$
  }

  const predicate = createEventFilterPredicate<T>(criteria)
  return <E extends BaseEvent<T>>(source$: Observable<E>) =>
    source$.pipe(filter(predicate as (event: E) => boolean))
}

/**
 * Builder class for creating complex filter criteria with a fluent API
 */
export class EventFilterBuilder<T = unknown> {
  private criteria: EventFilterCriteria<T> = {}

  /**
   * Filter by event category
   */
  byCategory(category: EventCategory): EventFilterBuilder<T> {
    this.criteria.category = category
    return this
  }

  /**
   * Filter by event type
   */
  byType(type: string): EventFilterBuilder<T> {
    this.criteria.type = type
    return this
  }

  /**
   * Filter by source ID
   */
  bySourceId(sourceId: string): EventFilterBuilder<T> {
    this.criteria.sourceId = sourceId
    return this
  }

  /**
   * Filter by source type
   */
  bySourceType(sourceType: string): EventFilterBuilder<T> {
    this.criteria.sourceType = sourceType
    return this
  }

  /**
   * Filter by minimum timestamp (events after this time)
   */
  since(timestamp: number): EventFilterBuilder<T> {
    this.criteria.since = timestamp
    return this
  }

  /**
   * Add a custom filter predicate for advanced filtering
   */
  where(predicate: (event: BaseEvent<T>) => boolean): EventFilterBuilder<T> {
    this.criteria.predicate = predicate
    return this
  }

  /**
   * Get the constructed filter criteria
   */
  build(): EventFilterCriteria<T> {
    return { ...this.criteria }
  }

  /**
   * Apply the filter to an array of events
   */
  applyToArray(events: BaseEvent<T>[]): BaseEvent<T>[] {
    return filterEvents(events, this.criteria)
  }

  /**
   * Apply the filter to an Observable of events
   */
  applyToStream(stream$: Observable<BaseEvent<T>>): Observable<BaseEvent<T>> {
    return filterEventStream(this.criteria)(stream$)
  }
}

/**
 * Create a new event filter builder with a fluent API
 */
export function createEventFilter<T = unknown>(): EventFilterBuilder<T> {
  return new EventFilterBuilder<T>()
}
