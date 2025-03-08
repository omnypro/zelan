import { BaseEvent as IBaseEvent, EventCategory } from '@s/types/events'

/**
 * Base class for all events
 */
export abstract class BaseEvent<T = unknown> implements IBaseEvent<T> {
  id: string
  timestamp: number
  source: string
  abstract category: EventCategory
  abstract type: string
  payload: T

  constructor(payload: T, source = 'system') {
    this.id = crypto.randomUUID()
    this.timestamp = Date.now()
    this.source = source
    this.payload = payload
  }
}
