/**
 * Event categories for grouping events
 */
export enum EventCategory {
  SYSTEM = 'system',
  ADAPTER = 'adapter',
  SERVICE = 'service',
  TWITCH = 'twitch',
  OBS = 'obs',
  USER = 'user'
}

/**
 * System event types
 */
export enum SystemEventType {
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  STARTUP = 'startup',
  SHUTDOWN = 'shutdown'
}

/**
 * Adapter event types
 */
export enum AdapterEventType {
  STATUS = 'status',
  CONNECTED = 'connected',
  DISCONNECTED = 'disconnected',
  ERROR = 'error',
  DATA = 'data'
}

// OBS events are now in ObsEvents.ts
export * from './ObsEvents'

/**
 * Base event interface with enhanced metadata
 */
export interface BaseEvent<T = unknown> {
  // Base identifier properties
  id: string
  timestamp: number

  // Categorization
  category: EventCategory
  type: string

  // Source metadata
  source: {
    id: string
    name: string
    type: string
  }

  // Main payload data
  data: T

  // App-specific metadata
  metadata?: {
    sessionId?: string
    correlationId?: string
    version?: string
    tags?: string[]
    ttl?: number // Time to live in seconds
  }

  // API response enrichment properties
  links?: Record<string, string> // HATEOAS-style links
}

/**
 * System information event payload
 */
export interface SystemInfoPayload {
  message: string
  level: 'info' | 'warning' | 'error'
  appVersion?: string
  details?: Record<string, unknown>
}

// OBS payload types are now in ObsEvents.ts

/**
 * Generic adapter status event payload
 */
export interface AdapterStatusPayload {
  status: string
  message?: string
  timestamp: number
  details?: Record<string, unknown>
}

// Event creation functions are now in src/shared/core/events
