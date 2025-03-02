// No import of uuid needed, BaseEvent already handles ID generation
import { BaseEvent } from './BaseEvent'
import { Event, EventCategory } from '@shared/types/events'

/**
 * Type for event constructor function
 */
export type EventConstructor<T, P = unknown> = new (payload: P) => T

/**
 * Type for event creator function
 */
export type EventCreator<P> = (payload: P) => Event<P>

/**
 * Create a factory function for a specific event type
 *
 * @param source Source of the event
 * @param category Event category
 * @param type Event type
 * @returns Factory function for creating events
 */
export function createEventCreator<P>(
  source: string,
  category: EventCategory,
  type: string
): EventCreator<P> {
  return (payload: P) => {
    return new BaseEvent<P>(source, category, type, payload)
  }
}

/**
 * Create a typed event class
 *
 * @param source Default source of events
 * @param category Event category
 * @param type Event type
 * @returns Event class constructor
 */
export function createEventClass<P>(
  source: string,
  category: EventCategory,
  type: string
): EventConstructor<Event<P>, P> {
  return class TypedEvent extends BaseEvent<P> {
    constructor(payload: P, customSource?: string) {
      super(customSource ?? source, category, type, payload)
    }
  }
}

/**
 * Create a factory for related events in the same category
 *
 * @param source Default source of events
 * @param category Event category
 * @returns Object with event creators
 */
export function createEventFactory<T extends Record<string, unknown>>(
  source: string,
  category: EventCategory
): {
  createEvent: <K extends keyof T>(type: K, payload: T[K]) => Event<T[K]>
  createEventCreator: <K extends keyof T>(type: K) => EventCreator<T[K]>
  createEventClass: <K extends keyof T>(type: K) => EventConstructor<Event<T[K]>, T[K]>
} {
  return {
    createEvent: <K extends keyof T>(type: K, payload: T[K]): Event<T[K]> => {
      return new BaseEvent<T[K]>(source, category, String(type), payload)
    },

    createEventCreator: <K extends keyof T>(type: K): EventCreator<T[K]> => {
      return (payload: T[K]) => {
        return new BaseEvent<T[K]>(source, category, String(type), payload)
      }
    },

    createEventClass: <K extends keyof T>(type: K): EventConstructor<Event<T[K]>, T[K]> => {
      return class TypedEvent extends BaseEvent<T[K]> {
        constructor(payload: T[K], customSource?: string) {
          super(customSource ?? source, category, String(type), payload)
        }
      }
    }
  }
}

/**
 * Create a system event
 *
 * @param type Event type
 * @param payload Event payload
 * @returns System event
 */
export function createSystemEvent<P>(type: string, payload: P): Event<P> {
  return new BaseEvent<P>('system', EventCategory.SYSTEM, type, payload)
}

/**
 * Create an adapter event
 *
 * @param adapterName Name of the adapter
 * @param type Event type
 * @param payload Event payload
 * @returns Adapter event
 */
export function createAdapterEvent<P>(adapterName: string, type: string, payload: P): Event<P> {
  return new BaseEvent<P>(adapterName, EventCategory.ADAPTER, type, payload)
}

/**
 * Create an auth event
 *
 * @param service Authentication service name
 * @param type Event type
 * @param payload Event payload
 * @returns Auth event
 */
export function createAuthEvent<P>(service: string, type: string, payload: P): Event<P> {
  return new BaseEvent<P>(service, EventCategory.AUTH, type, payload)
}
