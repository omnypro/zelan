import { BaseEvent as IBaseEvent, EventCategory, EventSource } from '@s/types/events'

/**
 * Base class for all events
 */
export abstract class BaseEvent<T = unknown> implements IBaseEvent<T> {
  id: string
  timestamp: number
  source: EventSource
  abstract category: EventCategory
  abstract type: string
  payload: T
  data: T
  metadata: {
    version: string
    [key: string]: unknown
  }

  constructor(payload: T, source: EventSource | string = 'system') {
    this.id = crypto.randomUUID()
    this.timestamp = Date.now()

    // Handle string or EventSource object
    if (typeof source === 'string') {
      this.source = {
        id: 'system',
        name: 'System',
        type: 'system'
      }
    } else {
      this.source = source
    }

    this.payload = payload
    this.data = payload // For compatibility with API spec
    this.metadata = {
      version: '1.0'
    }
  }
}
