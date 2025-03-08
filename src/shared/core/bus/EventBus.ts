import { Subject, Observable, filter, ReplaySubject } from 'rxjs';
import { BaseEvent, EventCategory } from '../../types/events';

/**
 * Core event bus for publishing and subscribing to events
 */
export class EventBus {
  // Making this public to make it accessible by tRPC
  public events$: Subject<BaseEvent<any>>;
  
  constructor() {
    // Use ReplaySubject to cache recent events for late subscribers
    this.events$ = new ReplaySubject<BaseEvent<any>>(100);
  }

  /**
   * Publish an event to the event bus
   * @param event The event to publish
   */
  publish<T>(event: BaseEvent<T>): void {
    this.events$.next(event);
  }

  /**
   * Get all events as an observable
   */
  getEvents(): Observable<BaseEvent<any>> {
    return this.events$.asObservable();
  }

  /**
   * Get events filtered by category
   * @param category The category to filter by
   */
  getEventsByCategory<T>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.events$.pipe(
      filter((event): event is BaseEvent<T> => event.category === category)
    );
  }

  /**
   * Get events filtered by category and type
   * @param category The category to filter by
   * @param type The type to filter by
   */
  getEventsByCategoryAndType<T>(category: EventCategory, type: string): Observable<BaseEvent<T>> {
    return this.events$.pipe(
      filter((event): event is BaseEvent<T> => 
        event.category === category && event.type === type
      )
    );
  }
}