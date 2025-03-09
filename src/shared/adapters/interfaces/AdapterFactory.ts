import { ServiceAdapter } from './ServiceAdapter'
import { EventBus } from '@s/core/bus/EventBus'

/**
 * Factory interface for creating adapter instances
 */
export interface AdapterFactory<T extends ServiceAdapter = ServiceAdapter> {
  /**
   * Type of the adapter this factory creates
   */
  readonly type: string

  /**
   * Create a new adapter instance
   * @param id Unique identifier for the adapter
   * @param name Human-readable name for the adapter
   * @param options Adapter-specific configuration options
   * @param eventBus Event bus instance to be used by the adapter
   */
  create(id: string, name: string, options: Record<string, any>, eventBus: EventBus): T
}
