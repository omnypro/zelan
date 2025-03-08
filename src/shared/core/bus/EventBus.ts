import { Observable, Subject } from 'rxjs'
import { BaseEvent, EventCategory } from '@s/types/events'
import { 
  EventFilterCriteria, 
  filterEventStream
} from '@s/utils/filters/event-filter'

/**
 * Interface for the event bus that handles event pub/sub
 */
export interface EventBus {
  /**
   * Observable of all events emitted by the event bus
   */
  readonly events$: Observable<BaseEvent>

  /**
   * Get events filtered by specified criteria
   */
  getFilteredEvents$<T = unknown>(criteria: EventFilterCriteria<T>): Observable<BaseEvent<T>>

  /**
   * Get events filtered by category
   */
  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>>

  /**
   * Get events filtered by type
   */
  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>>

  /**
   * Get events filtered by category and type
   */
  getEventsByCategoryAndType$<T = unknown>(
    category: EventCategory,
    type: string
  ): Observable<BaseEvent<T>>

  /**
   * Get events filtered by category (with payload type param)
   */
  getEventsByCategory<T = unknown>(category: EventCategory): Observable<BaseEvent<T>>

  /**
   * Get events filtered by type (with payload type param)
   */
  getEventsByType<T = unknown>(type: string): Observable<BaseEvent<T>>

  /**
   * Get events filtered by category and type (with payload type param)
   */
  getEventsByCategoryAndType<T = unknown>(
    category: EventCategory,
    type: string
  ): Observable<BaseEvent<T>>

  /**
   * Publish an event to all subscribers
   */
  publish(event: BaseEvent): void
}

/**
 * Base implementation of the EventBus interface
 */
export class BaseEventBus implements EventBus {
  private eventsSubject = new Subject<BaseEvent>()

  /**
   * Observable of all events emitted by the event bus
   */
  readonly events$ = this.eventsSubject.asObservable()

  /**
   * Get events filtered by specified criteria
   */
  getFilteredEvents$<T = unknown>(criteria: EventFilterCriteria<T>): Observable<BaseEvent<T>> {
    // Using type assertion to bridge the gap
    const stream$ = this.events$ as unknown as Observable<BaseEvent<T>>
    return filterEventStream<T>(criteria)(stream$)
  }

  /**
   * Get events filtered by category
   */
  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category })
  }

  /**
   * Get events filtered by type
   */
  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ type })
  }

  /**
   * Get events filtered by category and type
   */
  getEventsByCategoryAndType$<T = unknown>(
    category: EventCategory,
    type: string
  ): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category, type })
  }

  /**
   * Get events filtered by category (with payload type param)
   */
  getEventsByCategory<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.getEventsByCategory$<T>(category)
  }

  /**
   * Get events filtered by type (with payload type param)
   */
  getEventsByType<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.getEventsByType$<T>(type)
  }

  /**
   * Get events filtered by category and type (with payload type param)
   */
  getEventsByCategoryAndType<T = unknown>(
    category: EventCategory,
    type: string
  ): Observable<BaseEvent<T>> {
    return this.getEventsByCategoryAndType$<T>(category, type)
  }

  /**
   * Publish an event to all subscribers
   */
  publish(event: BaseEvent): void {
    this.eventsSubject.next(event)
  }
}