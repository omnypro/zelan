import { useMemo } from 'react';
import { Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { EventCategory } from '../../../shared/types/events';
import { createEventStream } from '../../../shared/core/bus/createEventStream';
import { rendererEventBus } from '../services/eventBus';
import { useObservable, useObservableWithStatus } from './useObservable';

/**
 * Create an event stream for a specific category and type
 *
 * @param category Event category
 * @param type Optional event type
 * @returns Event stream
 */
export function useEventStreamSource<T = unknown>(
  category: EventCategory,
  type?: string
) {
  return useMemo(() => {
    return createEventStream<T>(rendererEventBus, category, type);
  }, [category, type]);
}

/**
 * Subscribe to an event stream and get the events
 *
 * @param category Event category
 * @param type Optional event type
 * @returns Array of events
 */
export function useEvents<T = unknown>(
  category: EventCategory,
  type?: string,
  limit: number = 100
): Array<{ id: string; timestamp: number; payload: T }> {
  const stream$ = useEventStreamSource<T>(category, type);

  const events$ = useMemo(() => {
    return new Observable<Array<{ id: string; timestamp: number; payload: T }>>((observer) => {
      const events: Array<{ id: string; timestamp: number; payload: T }> = [];

      const subscription = stream$.subscribe({
        next: (event) => {
          events.unshift({
            id: event.id,
            timestamp: event.timestamp,
            payload: event.payload
          });

          // Keep only the latest events
          if (events.length > limit) {
            events.pop();
          }

          observer.next([...events]);
        },
        error: (err) => observer.error(err),
        complete: () => observer.complete()
      });

      // Initial empty array
      observer.next([]);

      return () => subscription.unsubscribe();
    });
  }, [stream$, limit]);

  return useObservable(events$, []);
}

/**
 * Subscribe to event payloads
 *
 * @param category Event category
 * @param type Optional event type
 * @returns Current event payload
 */
export function useEventPayload<T = unknown>(
  category: EventCategory,
  type: string,
  initialValue?: T
): {
  payload: T | undefined;
  loading: boolean;
  error: Error | null;
} {
  const stream$ = useEventStreamSource<T>(category, type);
  const payloads$ = useMemo(() => stream$.pipe(map((event) => event.payload)), [stream$]);

  const result = useObservableWithStatus(payloads$, initialValue);

  return {
    payload: result.value,
    loading: result.loading,
    error: result.error
  };
}

/**
 * Create a function to publish events of a specific category and type
 *
 * @param category Event category
 * @param type Event type
 * @returns Function to publish events with the specified category and type
 */
export function useEventPublisher<T = unknown>(
  category: EventCategory,
  type: string,
  source: string = 'ui'
): (payload: T) => void {
  return useMemo(() => {
    return (payload: T) => {
      rendererEventBus.publish({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        source,
        category,
        type,
        payload
      });
    };
  }, [category, type, source]);
}