import { EventBus } from '@s/core/bus'
import { ServiceAdapter } from '../interfaces/ServiceAdapter'

/**
 * Type for adapter creation functions
 */
export type AdapterCreator = (
  id: string,
  name: string,
  options: Record<string, unknown>,
  eventBus: EventBus
) => ServiceAdapter

/**
 * Registry for adapter creators
 */
export class AdapterRegistry {
  private creators: Map<string, AdapterCreator> = new Map()

  /**
   * Register an adapter creator function
   * @param type The adapter type
   * @param creator Function to create adapters of this type
   */
  register(type: string, creator: AdapterCreator): void {
    this.creators.set(type, creator)
  }

  /**
   * Create an adapter of the specified type
   * @param type The adapter type
   * @param id Unique adapter ID
   * @param name Human-readable adapter name
   * @param options Adapter configuration options
   * @param eventBus Event bus instance
   */
  createAdapter(
    type: string,
    id: string,
    name: string,
    options: Record<string, unknown>,
    eventBus: EventBus
  ): ServiceAdapter {
    const creator = this.creators.get(type)
    if (!creator) {
      throw new Error(`No creator registered for adapter type ${type}`)
    }

    return creator(id, name, options, eventBus)
  }

  /**
   * Check if a creator for the given type is registered
   * @param type The adapter type
   */
  hasCreator(type: string): boolean {
    return this.creators.has(type)
  }

  /**
   * Get all registered adapter types
   */
  getAdapterTypes(): string[] {
    return Array.from(this.creators.keys())
  }
}
