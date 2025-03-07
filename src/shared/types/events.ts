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

/**
 * Base event interface
 */
export interface BaseEvent<T = unknown> {
  id: string;
  timestamp: number;
  source: string;
  category: EventCategory;
  type: string;
  payload: T;
}

/**
 * System information event payload
 */
export interface SystemInfoPayload {
  message?: string;
  appVersion?: string;
}