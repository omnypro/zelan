/**
 * Base event interface that all events implement
 */
export interface Event<T = unknown> {
  /**
   * Unique identifier for the event
   */
  id: string;
  
  /**
   * Timestamp when the event was created
   */
  timestamp: number;
  
  /**
   * Source of the event (adapter name, system, etc.)
   */
  source: string;
  
  /**
   * Event category for grouping related events
   */
  category: EventCategory;
  
  /**
   * Specific event type within the category
   */
  type: string;
  
  /**
   * Event payload containing the actual data
   */
  payload: T;
}

/**
 * Event categories for organizing events
 */
export enum EventCategory {
  SYSTEM = 'system',
  ADAPTER = 'adapter',
  AUTH = 'auth',
  UI = 'ui',
  WEBSOCKET = 'websocket',
  TWITCH = 'twitch',
  OBS = 'obs',
  TEST = 'test',
}

/**
 * System event types
 */
export enum SystemEventType {
  STARTUP = 'startup',
  SHUTDOWN = 'shutdown',
  ERROR = 'error',
  WARNING = 'warning',
  INFO = 'info',
}

/**
 * Authentication event types
 */
export enum AuthEventType {
  LOGIN_STARTED = 'login_started',
  LOGIN_SUCCESS = 'login_success',
  LOGIN_FAILED = 'login_failed',
  LOGOUT = 'logout',
  TOKEN_REFRESH = 'token_refresh',
  TOKEN_EXPIRED = 'token_expired',
}

/**
 * Adapter event types
 */
export enum AdapterEventType {
  CONNECTED = 'connected',
  DISCONNECTED = 'disconnected',
  RECONNECTING = 'reconnecting',
  DATA_RECEIVED = 'data_received',
  ERROR = 'error',
  STATUS_CHANGED = 'status_changed',
}

/**
 * WebSocket event types
 */
export enum WebSocketEventType {
  CLIENT_CONNECTED = 'client_connected',
  CLIENT_DISCONNECTED = 'client_disconnected',
  MESSAGE_RECEIVED = 'message_received',
  MESSAGE_SENT = 'message_sent',
  ERROR = 'error',
}

/**
 * System event payload types
 */
export interface SystemStartupPayload {
  appVersion: string;
  startTime: number;
}

export interface SystemErrorPayload {
  error: string;
  code?: string | number;
  stack?: string;
}

/**
 * Authentication event payload types
 */
export interface AuthPayload {
  service: string;
  userId?: string;
}

export interface AuthSuccessPayload extends AuthPayload {
  expiresAt: number;
  scopes: string[];
}

export interface AuthErrorPayload extends AuthPayload {
  error: string;
  code?: string;
}

/**
 * Type guard to check if an object is a valid Event
 */
export function isEvent(obj: unknown): obj is Event {
  if (!obj || typeof obj !== 'object') {
    return false;
  }
  
  const event = obj as Record<string, unknown>;
  
  return (
    typeof event.id === 'string' &&
    typeof event.timestamp === 'number' &&
    typeof event.source === 'string' &&
    typeof event.category === 'string' &&
    typeof event.type === 'string' &&
    'payload' in event
  );
}

/**
 * Type union for all event payloads
 */
export type EventPayload = 
  | SystemStartupPayload
  | SystemErrorPayload
  | AuthPayload
  | AuthSuccessPayload
  | AuthErrorPayload
  | unknown;