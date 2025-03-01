import { Observable, OperatorFunction } from 'rxjs'
import { map } from 'rxjs/operators'
import { Event, EventCategory } from '@shared/types/events'
import { IEventBus } from './EventBus'

/**
 * Interface for a typed event stream
 */
export interface EventStream<T = unknown> extends Observable<Event<T>> {
  /**
   * Map events to their payloads
   *
   * @returns Observable of event payloads
   */
  payloads(): Observable<T>

  /**
   * Apply a filter to the event stream
   *
   * @param predicate Filter function
   * @returns Filtered event stream
   */
  filter(predicate: (event: Event<T>) => boolean): EventStream<T>

  /**
   * Apply RxJS operators to the event stream
   *
   * @param operators RxJS operators to apply
   * @returns Transformed observable
   */
  pipe<R>(...operators: OperatorFunction<Event<T>, R>[]): Observable<R>
}

/**
 * Create a typed event stream for a specific category and optional type
 *
 * @param eventBus Event bus to create stream from
 * @param category Event category to filter
 * @param type Optional event type to filter
 * @returns Typed event stream
 */
export function createEventStream<T = unknown>(
  eventBus: IEventBus,
  category: EventCategory,
  type?: string
): EventStream<T> {
  // Create base observable from the event bus
  const baseStream = type ? eventBus.of<T>(category, type) : eventBus.ofCategory<T>(category)

  // Extend the observable with EventStream methods
  const stream = baseStream as EventStream<T>

  // Add payloads method
  stream.payloads = function (): Observable<T> {
    return this.pipe(map((event) => event.payload))
  }

  // Override filter method to return EventStream
  const originalFilter = stream.filter
  stream.filter = function (predicate: (event: Event<T>) => boolean): EventStream<T> {
    const filtered = originalFilter.call(this, predicate) as EventStream<T>

    // Add payloads method to filtered stream
    filtered.payloads = function (): Observable<T> {
      return this.pipe(map((event) => event.payload))
    }

    return filtered
  }

  return stream
}

/**
 * Create a typed event stream for a specific event class
 *
 * @param eventBus Event bus to create stream from
 * @param eventClass Event class to filter by
 * @returns Typed event stream
 */
export function createEventClassStream<T, E extends Event<T>>(
  eventBus: IEventBus,
  eventClass: { new (...args: unknown[]): E }
): EventStream<T> {
  // Create an instance to determine category and type
  const instance = new eventClass()

  // Create stream from category and type
  const stream = createEventStream<T>(eventBus, instance.category, instance.type)

  return stream
}

/**
 * Export index of event stream creation functions
 */
export default {
  createEventStream,
  createEventClassStream
}
