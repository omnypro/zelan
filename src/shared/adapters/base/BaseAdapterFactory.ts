import { AdapterFactory } from '../interfaces/AdapterFactory'
import { ServiceAdapter } from '../interfaces/ServiceAdapter'
import { EventBus } from '@s/core/bus/EventBus'

/**
 * Abstract base class for adapter factories
 */
export abstract class BaseAdapterFactory<T extends ServiceAdapter = ServiceAdapter>
  implements AdapterFactory<T>
{
  readonly type: string

  constructor(type: string) {
    this.type = type
  }

  /**
   * Create a new adapter instance
   */
  abstract create(id: string, name: string, options: Record<string, any>, eventBus: EventBus): T
}
