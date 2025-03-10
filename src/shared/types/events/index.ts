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

// Import OBS events
export * from './ObsEvents'

/**
 * Event source information
 */
export interface EventSource {
  id: string
  name: string
  type: string
}

/**
 * Base event interface
 */
export interface BaseEvent<T = unknown> {
  // Base identifier properties
  id: string
  timestamp: number

  // Categorization
  category: EventCategory
  type: string

  // Source information
  source: EventSource

  // Main payload data
  data: T

  // App-specific metadata
  metadata: {
    version: string
    [key: string]: unknown
  }
}

/**
 * System information event payload
 */
export interface SystemInfoPayload {
  message: string
  level: 'info' | 'warning' | 'error'
  details?: Record<string, unknown>
}

/**
 * Generic adapter status event payload
 */
export interface AdapterStatusPayload {
  status: string
  message?: string
  timestamp: number
  details?: Record<string, unknown>
}
