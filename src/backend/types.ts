/**
 * Common TypeScript types for Zelan
 */

/**
 * Standardized event structure for all events flowing through the system
 */
export interface StreamEvent {
  /** Source service that generated the event (e.g., "obs", "twitch") */
  source: string;
  /** Type of event (e.g., "scene.changed", "chat.message") */
  event_type: string;
  /** Arbitrary JSON payload with event details */
  payload: any;
  /** Timestamp when the event was created */
  timestamp: string;
}

/**
 * Statistics about event bus activity
 */
export interface EventBusStats {
  events_published: number;
  events_dropped: number;
  source_counts: Record<string, number>;
  type_counts: Record<string, number>;
}

/**
 * State for managing service connection
 */
export enum ServiceStatus {
  Disconnected = 'Disconnected',
  Connecting = 'Connecting',
  Connected = 'Connected',
  Error = 'Error',
  Disabled = 'Disabled'
}

/**
 * Settings for a service adapter
 */
export interface AdapterSettings {
  /** Whether the adapter is enabled */
  enabled: boolean;
  /** Adapter-specific configuration */
  config: any;
  /** Display name for the adapter */
  display_name: string;
  /** Description of the adapter's functionality */
  description: string;
}

/**
 * Base configuration for all adapters
 */
export interface BaseAdapterConfig {
  [key: string]: any;
}

/**
 * Configuration for the TestAdapter
 */
export interface TestAdapterConfig extends BaseAdapterConfig {
  /** Interval between events in milliseconds */
  interval_ms: number;
  /** Whether to generate special events */
  generate_special_events: boolean;
}

/**
 * WebSocket configuration
 */
export interface WebSocketConfig {
  /** Port to bind the WebSocket server to */
  port: number;
}

/**
 * Error information
 */
export interface ZelanError {
  /** Error code */
  code: string;
  /** Error message */
  message: string;
  /** Additional context information */
  context?: string;
  /** Error severity */
  severity: 'info' | 'warning' | 'error' | 'critical';
}