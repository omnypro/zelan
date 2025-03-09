import { useMemo } from 'react'
import { Observable } from 'rxjs'
import { map } from 'rxjs/operators'
import { EventCategory } from '@s/types/events'
import { createEventStream, createFilteredEventStream } from '@s/core/bus/createEventStream'
import { rendererEventBus } from '@r/services/eventBus'
import { useObservable, useObservableWithStatus } from './useObservable'
import { EventFilterCriteria } from '@s/utils/filters/event-filter'

/**
 * Create an event stream for a specific category and type
 *
 * @param category Event category
 * @param type Optional event type
 * @returns Event stream
 */
export function useEventStreamSource<T = unknown>(category: EventCategory, type?: string) {
  return useMemo(() => {
    return createEventStream<T>(rendererEventBus, category, type)
  }, [category, type])
}

/**
 * Create an event stream with custom filter criteria
 *
 * @param filterCriteria Filter criteria to apply
 * @returns Filtered event stream
 */
export function useFilteredEventStream<T = unknown>(filterCriteria: EventFilterCriteria<T>) {
  return useMemo(() => {
    return createFilteredEventStream<T>(rendererEventBus, filterCriteria)
  }, [filterCriteria])
}

/**
 * Subscribe to an event stream and get the events
 *
 * @param category Event category
 * @param type Optional event type
 * @param limit Maximum number of events to keep
 * @returns Array of events
 */
export function useEvents<T = unknown>(
  category: EventCategory,
  type?: string,
  limit: number = 100
): Array<{ id: string; timestamp: number; payload: T }> {
  const stream$ = useEventStreamSource<T>(category, type)

  const events$ = useMemo(() => {
    return new Observable<Array<{ id: string; timestamp: number; payload: T }>>((observer) => {
      const events: Array<{ id: string; timestamp: number; payload: T }> = []

      const subscription = stream$.subscribe({
        next: (event) => {
          events.unshift({
            id: event.id,
            timestamp: event.timestamp,
            payload: event.payload as T
          })

          // Keep only the latest events
          if (events.length > limit) {
            events.pop()
          }

          observer.next([...events])
        },
        error: (err) => observer.error(err),
        complete: () => observer.complete()
      })

      // Initial empty array
      observer.next([])

      return () => subscription.unsubscribe()
    })
  }, [stream$, limit])

  return useObservable(events$, [])
}

/**
 * Subscribe to events with custom filter criteria
 *
 * @param filterCriteria Filter criteria to apply
 * @param limit Maximum number of events to keep
 * @returns Array of filtered events
 */
export function useFilteredEvents<T = unknown>(
  filterCriteria: EventFilterCriteria<T>,
  limit: number = 100
): Array<{ id: string; timestamp: number; payload: T }> {
  const stream$ = useFilteredEventStream<T>(filterCriteria)

  const events$ = useMemo(() => {
    return new Observable<Array<{ id: string; timestamp: number; payload: T }>>((observer) => {
      const events: Array<{ id: string; timestamp: number; payload: T }> = []

      const subscription = stream$.subscribe({
        next: (event) => {
          events.unshift({
            id: event.id,
            timestamp: event.timestamp,
            payload: event.payload as T
          })

          // Keep only the latest events
          if (events.length > limit) {
            events.pop()
          }

          observer.next([...events])
        },
        error: (err) => observer.error(err),
        complete: () => observer.complete()
      })

      // Initial empty array
      observer.next([])

      return () => subscription.unsubscribe()
    })
  }, [stream$, limit])

  return useObservable(events$, [])
}

/**
 * Subscribe to event payloads
 *
 * @param category Event category
 * @param type Optional event type
 * @param initialValue Initial value to use before first event
 * @returns Current event payload with loading and error status
 */
export function useEventPayload<T = unknown>(
  category: EventCategory,
  type: string,
  initialValue?: T
): {
  payload: T | undefined
  loading: boolean
  error: Error | null
} {
  const stream$ = useEventStreamSource<T>(category, type)
  const payloads$ = useMemo(() => stream$.pipe(map((event) => event.payload)), [stream$])

  const result = useObservableWithStatus(payloads$, initialValue)

  return {
    payload: result.value,
    loading: result.loading,
    error: result.error
  }
}

/**
 * Subscribe to event payloads with custom filter criteria
 *
 * @param filterCriteria Filter criteria to apply
 * @param initialValue Initial value to use before first event
 * @returns Current event payload with loading and error status
 */
export function useFilteredEventPayload<T = unknown>(
  filterCriteria: EventFilterCriteria<T>,
  initialValue?: T
): {
  payload: T | undefined
  loading: boolean
  error: Error | null
} {
  const stream$ = useFilteredEventStream<T>(filterCriteria)
  const payloads$ = useMemo(() => stream$.pipe(map((event) => event.payload)), [stream$])

  const result = useObservableWithStatus(payloads$, initialValue)

  return {
    payload: result.value,
    loading: result.loading,
    error: result.error
  }
}

/**
 * Create a function to publish events of a specific category and type
 *
 * @param category Event category
 * @param type Event type
 * @param source Source identifier
 * @returns Function to publish events with the specified category and type
 */
export function useEventPublisher<T = unknown>(
  category: EventCategory,
  type: string,
  source = 'ui'
): (payload: T) => void {
  return useMemo(() => {
    return (payload: T) => {
      rendererEventBus.publish({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        source: { id: source, type: 'ui' },
        category,
        type,
        payload
      })
    }
  }, [category, type, source])
}
