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
 * Event source information
 */
export interface EventSource {
  id: string;
  name: string;
  type: string;
}

/**
 * Base event interface
 */
export interface BaseEvent<T = unknown> {
  id: string;
  timestamp: number;
  source: EventSource;
  category: EventCategory;
  type: string;
  data: T;
  metadata: {
    version: string;
    [key: string]: unknown;
  };
  
  // For compatibility with older code
  payload?: T;
}

/**
 * System information event payload
 */
export interface SystemInfoPayload {
  message?: string;
  appVersion?: string;
}