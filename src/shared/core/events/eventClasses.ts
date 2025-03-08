import { EventCategory, SystemEventType, SystemInfoPayload, BaseEvent } from '@s/types/events'
import { createSystemEvent, createObsEvent, createEvent } from './createEvents'
import { ObsEventType } from '@s/types/events/ObsEvents'

/**
 * Base class for all event types
 */
export class Event<T> implements BaseEvent<T> {
  id: string
  timestamp: number
  category: EventCategory
  type: string
  source: {
    id: string
    name: string
    type: string
  }
  data: T
  metadata: {
    version: string
    [key: string]: unknown
  }

  // For backward compatibility
  get payload(): T {
    return this.data
  }

  constructor(
    category: EventCategory,
    type: string,
    data: T,
    sourceId: string,
    sourceName: string = sourceId,
    sourceType: string = category
  ) {
    const event = createEvent(category, type, data, sourceId, sourceName, sourceType)
    this.id = event.id
    this.timestamp = event.timestamp
    this.category = event.category
    this.type = event.type
    this.source = event.source
    this.data = event.data
    this.metadata = event.metadata
  }
}

/**
 * System information event class
 */
export class SystemInfoEvent extends Event<SystemInfoPayload> {
  constructor(
    message: string,
    level: 'info' | 'warning' | 'error' = 'info',
    details?: Record<string, unknown>
  ) {
    const event = createSystemEvent(SystemEventType.INFO, message, level, details)
    super(
      event.category,
      event.type,
      event.data,
      event.source.id,
      event.source.name,
      event.source.type
    )
  }
}

/**
 * System error event class
 */
export class SystemErrorEvent extends Event<SystemInfoPayload> {
  constructor(message: string, details?: Record<string, unknown>) {
    const event = createSystemEvent(SystemEventType.ERROR, message, 'error', details)
    super(
      event.category,
      event.type,
      event.data,
      event.source.id,
      event.source.name,
      event.source.type
    )
  }
}

/**
 * OBS event class
 */
export class ObsEvent<T> extends Event<T> {
  constructor(type: ObsEventType, data: T, adapterId: string, adapterName: string) {
    const event = createObsEvent(type, data, adapterId, adapterName)
    super(
      event.category,
      event.type,
      event.data,
      event.source.id,
      event.source.name,
      event.source.type
    )
  }
}

/**
 * Adapter event class
 */
export class AdapterEvent<T> extends Event<T> {
  constructor(type: string, data: T, adapterId: string, adapterName: string, adapterType: string) {
    super(EventCategory.ADAPTER, type, data, adapterId, adapterName, adapterType)
  }
}
