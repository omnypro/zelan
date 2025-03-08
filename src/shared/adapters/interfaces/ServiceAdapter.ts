import { Observable } from 'rxjs'
import { AdapterStatusInfo } from './AdapterStatus'
import { EventBus } from '@s/core/bus/EventBus'

/**
 * Basic adapter configuration object
 */
export interface AdapterConfig {
  id: string
  type: string
  name: string
  enabled: boolean
  options: Record<string, unknown>
}

/**
 * Interface for all service adapters
 */
export interface ServiceAdapter {
  /**
   * Unique identifier for the adapter instance
   */
  readonly id: string

  /**
   * Type of the adapter (e.g., 'twitch', 'obs')
   */
  readonly type: string

  /**
   * Human-readable name for the adapter
   */
  readonly name: string

  /**
   * Whether the adapter is enabled
   */
  readonly enabled: boolean

  /**
   * Adapter-specific configuration options
   */
  readonly options: Record<string, any>

  /**
   * Event bus instance used by this adapter
   */
  readonly eventBus: EventBus

  /**
   * Observable of adapter status changes
   */
  readonly status$: Observable<AdapterStatusInfo>

  /**
   * Initialize the adapter
   */
  initialize(): Promise<void>

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
   * Update adapter configuration
   * @param config Updated configuration
   */
  updateConfig(config: Partial<AdapterConfig>): Promise<void>

  /**
   * Dispose of resources used by the adapter
   */
  dispose(): Promise<void>
}
