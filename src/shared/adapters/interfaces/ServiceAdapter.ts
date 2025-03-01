import { Observable } from 'rxjs'
import { Event } from '@shared/types/events'
import { AdapterStatus, AdapterStatusInfo } from './AdapterStatus'

/**
 * Configuration for an adapter
 */
export interface AdapterConfig {
  /**
   * Unique ID for the adapter instance
   */
  id: string

  /**
   * Adapter type identifier
   */
  type: string

  /**
   * User-friendly name for the adapter
   */
  name: string

  /**
   * Whether the adapter should be enabled
   */
  enabled: boolean

  /**
   * Whether the adapter should automatically connect on startup
   */
  autoConnect: boolean

  /**
   * Whether the adapter should attempt to reconnect on disconnection
   */
  autoReconnect: boolean

  /**
   * Maximum number of reconnection attempts (-1 for infinite)
   */
  maxReconnectAttempts: number

  /**
   * Adapter-specific settings
   */
  settings: Record<string, unknown>
}

/**
 * Interface that all service adapters must implement
 */
export interface ServiceAdapter {
  /**
   * Unique ID for this adapter instance
   */
  readonly id: string

  /**
   * Type identifier for this adapter
   */
  readonly type: string

  /**
   * User-friendly name for this adapter
   */
  readonly name: string

  /**
   * Current status of the adapter
   */
  readonly status: AdapterStatus

  /**
   * Extended status information
   */
  readonly statusInfo: AdapterStatusInfo

  /**
   * Observable that emits the adapter's status when it changes
   */
  readonly status$: Observable<AdapterStatusInfo>

  /**
   * Observable that emits events from this adapter
   */
  readonly events$: Observable<Event>

  /**
   * Whether this adapter is currently connected
   */
  readonly isConnected: boolean

  /**
   * Whether this adapter is currently enabled
   */
  readonly isEnabled: boolean

  /**
   * Initialize the adapter with configuration
   * @param config Adapter configuration
   */
  initialize(config: AdapterConfig): Promise<void>

  /**
   * Connect to the service
   */
  connect(): Promise<void>

  /**
   * Disconnect from the service
   */
  disconnect(): Promise<void>

  /**
   * Reconnect to the service
   */
  reconnect(): Promise<void>

  /**
   * Update the adapter configuration
   * @param config New adapter configuration
   */
  updateConfig(config: Partial<AdapterConfig>): Promise<void>

  /**
   * Get the current adapter configuration
   */
  getConfig(): AdapterConfig

  /**
   * Enable the adapter
   */
  enable(): Promise<void>

  /**
   * Disable the adapter
   */
  disable(): Promise<void>

  /**
   * Dispose of the adapter and clean up resources
   */
  dispose(): Promise<void>
}
