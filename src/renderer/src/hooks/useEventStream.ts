import { useMemo } from 'react'
import { map, scan, shareReplay } from 'rxjs/operators'
import { EventCategory, BaseEvent } from '@s/types/events'
import { createEventStream, createFilteredEventStream } from '@s/core/bus/createEventStream'
import { rendererEventBus } from '@r/services/eventBus'
import { useObservable, useObservableState } from './useObservable'
import { EventFilterCriteria } from '@s/core/bus'

/**
 * Simplified event record
 */
export interface EventRecord<T> {
  id: string
  timestamp: number
  data: T
  category?: EventCategory
  type?: string
  source?: {
    id: string
    name: string
    type: string
  }
}

/**
 * Get a shared, cached event stream for a specific category and type
 */
export function useEventStream<T = unknown>(category: EventCategory, type?: string) {
  return useMemo(
    () => createEventStream<T>(rendererEventBus, category, type).pipe(shareReplay({ bufferSize: 1, refCount: true })),
    [category, type]
  )
}

/**
 * Get a shared, cached event stream with custom filter criteria
 */
export function useFilteredStream<T = unknown>(filterCriteria: EventFilterCriteria<T>) {
  return useMemo(
    () =>
      createFilteredEventStream<T>(rendererEventBus, filterCriteria).pipe(
        shareReplay({ bufferSize: 1, refCount: true })
      ),
    [filterCriteria]
  )
}

/**
 * Transform a stream of events into an accumulating array with a size limit
 */
function accumulateEvents<T>(
  stream$: ReturnType<typeof useEventStream<T> | typeof useFilteredStream<T>>,
  limit: number
) {
  return useMemo(
    () =>
      stream$.pipe(
        // Transform events to simplified format
        map((event: BaseEvent<T>) => ({
          id: event.id,
          timestamp: event.timestamp,
          data: event.data,
          category: event.category,
          type: event.type,
          source: event.source
        })),
        // Accumulate events with newest first
        scan((acc: EventRecord<T>[], event: EventRecord<T>) => {
          const newEvents = [event, ...acc]
          return newEvents.slice(0, limit)
        }, [] as EventRecord<T>[]),
        // Share accumulated state
        shareReplay(1)
      ),
    [stream$, limit]
  )
}

/**
 * Get a list of accumulated events for a specific category and type
 */
export function useEvents<T = unknown>(category: EventCategory, type?: string, limit: number = 100): EventRecord<T>[] {
  const stream$ = useEventStream<T>(category, type)
  const events$ = accumulateEvents<T>(stream$, limit)
  return useObservable(events$, [])
}

/**
 * Get a list of accumulated events with custom filter criteria
 */
export function useFilteredEvents<T = unknown>(
  filterCriteria: EventFilterCriteria<T>,
  limit: number = 100
): EventRecord<T>[] {
  const stream$ = useFilteredStream<T>(filterCriteria)
  const events$ = accumulateEvents<T>(stream$, limit)
  return useObservable(events$, [])
}

/**
 * Get the latest event data with loading/error state
 */
export function useLatestEvent<T = unknown>(category: EventCategory, type: string, initialValue?: T) {
  const stream$ = useEventStream<T>(category, type)
  const data$ = useMemo(() => stream$.pipe(map((event) => event.data)), [stream$])

  return useObservableState(data$, initialValue)
}

/**
 * Get the latest event data with custom filter criteria
 */
export function useLatestFilteredEvent<T = unknown>(filterCriteria: EventFilterCriteria<T>, initialValue?: T) {
  const stream$ = useFilteredStream<T>(filterCriteria)
  const data$ = useMemo(() => stream$.pipe(map((event) => event.data)), [stream$])

  return useObservableState(data$, initialValue)
}

/**
 * Create a function to publish events of a specific category and type
 */
export function useEventPublisher<T = unknown>(category: EventCategory, type: string, source = 'ui') {
  return useMemo(
    () => (data: T) => {
      rendererEventBus.publish({
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        source: { id: source, type: 'ui', name: source },
        category,
        type,
        data,
        metadata: {
          version: '1.0'
        }
      })
    },
    [category, type, source]
  )
}
