import { Observable, Subject, ReplaySubject, Subscription, Observer, OperatorFunction } from 'rxjs'
import { filter, share, map } from 'rxjs/operators'
import { Event, EventCategory, isEvent } from '@shared/types/events'
import { BaseEvent } from '@shared/core/events/BaseEvent'

/**
 * Interface for the event bus
 */
export interface IEventBus {
  /**
   * Publish an event to the bus
   *
   * @param event The event to publish
   */
  publish<T>(event: Event<T>): void

  /**
   * Create an observable of all events
   *
   * @returns Observable of all events
   */
  events$(): Observable<Event>

  /**
   * Create an observable of events filtered by category
   *
   * @param category The category to filter by
   * @returns Observable of events in the category
   */
  ofCategory<T = unknown>(category: EventCategory): Observable<Event<T>>

  /**
   * Create an observable of events filtered by type
   *
   * @param type The event type to filter by
   * @returns Observable of events of the specified type
   */
  ofType<T = unknown>(type: string): Observable<Event<T>>

  /**
   * Create an observable of events filtered by category and type
   *
   * @param category The category to filter by
   * @param type The event type to filter by
   * @returns Observable of events matching both category and type
   */
  of<T = unknown>(category: EventCategory, type: string): Observable<Event<T>>

  /**
   * Subscribe to events with a callback
   *
   * @param observer Observer or next callback
   * @returns Subscription that can be unsubscribed
   */
  subscribe(observer: Partial<Observer<Event>> | ((event: Event) => void)): Subscription
}

/**
 * Core implementation of the event bus
 */
export class EventBus implements IEventBus {
  /**
   * Subject that emits all events
   */
  private readonly eventSubject: Subject<Event> = new ReplaySubject<Event>(100)

  /**
   * Shared observable of all events
   */
  private readonly events: Observable<Event>

  /**
   * Create a new event bus
   */
  constructor() {
    this.events = this.eventSubject.pipe(share())
  }

  /**
   * Publish an event to the bus
   *
   * @param event The event to publish
   */
  public publish<T>(event: Event<T>): void {
    if (!isEvent(event)) {
      console.error('Invalid event:', event)
      return
    }

    // Convert to BaseEvent if it's a plain object
    const baseEvent = event instanceof BaseEvent ? event : BaseEvent.fromObject(event)

    this.eventSubject.next(baseEvent)
  }

  /**
   * Create an observable of all events
   *
   * @returns Observable of all events
   */
  public events$(): Observable<Event> {
    return this.events
  }

  /**
   * Create an observable of events filtered by category
   *
   * @param category The category to filter by
   * @returns Observable of events in the category
   */
  public ofCategory<T = unknown>(category: EventCategory): Observable<Event<T>> {
    return this.events.pipe(filter((event): event is Event<T> => event.category === category))
  }

  /**
   * Create an observable of events filtered by type
   *
   * @param type The event type to filter by
   * @returns Observable of events of the specified type
   */
  public ofType<T = unknown>(type: string): Observable<Event<T>> {
    return this.events.pipe(filter((event): event is Event<T> => event.type === type))
  }

  /**
   * Create an observable of events filtered by category and type
   *
   * @param category The category to filter by
   * @param type The event type to filter by
   * @returns Observable of events matching both category and type
   */
  public of<T = unknown>(category: EventCategory, type: string): Observable<Event<T>> {
    return this.events.pipe(
      filter((event): event is Event<T> => event.category === category && event.type === type)
    )
  }

  /**
   * Create an observable of events filtered by predicate
   *
   * @param predicate Function that tests each event
   * @returns Observable of events that pass the test
   */
  public filter<T = unknown>(predicate: (event: Event) => boolean): Observable<Event<T>> {
    return this.events.pipe(filter((event): event is Event<T> => predicate(event)))
  }

  /**
   * Apply transformations to the event stream using a single operator
   *
   * @param operator The operator to apply
   * @returns Transformed observable
   */
  public pipe<R>(operator: OperatorFunction<Event, R>): Observable<R> {
    return this.events.pipe(operator)
  }

  /**
   * Get an Observable that can be transformed with standard RxJS operators
   *
   * @returns Observable of events
   */
  public asObservable(): Observable<Event> {
    return this.events
  }

  /**
   * Extract the payload from events
   *
   * @param category The category to filter by
   * @param type The event type to filter by
   * @returns Observable of payloads
   */
  public payloads<T = unknown>(category: EventCategory, type: string): Observable<T> {
    return this.of<T>(category, type).pipe(map((event) => event.payload))
  }

  /**
   * Subscribe to events with a callback
   *
   * @param observer Observer or next callback
   * @returns Subscription that can be unsubscribed
   */
  public subscribe(observer: Partial<Observer<Event>> | ((event: Event) => void)): Subscription {
    if (typeof observer === 'function') {
      return this.events.subscribe({
        next: observer
      })
    }

    return this.events.subscribe(observer)
  }
}
