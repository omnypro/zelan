import { Observable, Subject } from 'rxjs'
import { filter } from 'rxjs/operators'
import { BaseEvent, EventCategory } from '@s/types/events'

/**
 * Interface for the event bus that handles event pub/sub
 */
export interface EventBus {
  /**
   * Observable of all events emitted by the event bus
   */
  readonly events$: Observable<BaseEvent>

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
   * Get events filtered by category
   */
  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.events$.pipe(filter((event) => event.category === category)) as Observable<
      BaseEvent<T>
    >
  }

  /**
   * Get events filtered by type
   */
  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.events$.pipe(filter((event) => event.type === type)) as Observable<BaseEvent<T>>
  }

  /**
   * Get events filtered by category and type
   */
  getEventsByCategoryAndType$<T = unknown>(
    category: EventCategory,
    type: string
  ): Observable<BaseEvent<T>> {
    return this.events$.pipe(
      filter((event) => event.category === category && event.type === type)
    ) as Observable<BaseEvent<T>>
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
