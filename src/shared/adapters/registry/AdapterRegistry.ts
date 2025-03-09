import { AdapterFactory } from '../interfaces/AdapterFactory'

/**
 * Registry for adapter factories
 */
export class AdapterRegistry {
  private factories: Map<string, AdapterFactory> = new Map()

  /**
   * Register an adapter factory
   * @param factory The factory to register
   */
  register(factory: AdapterFactory): void {
    this.factories.set(factory.type, factory)
  }

  /**
   * Get an adapter factory by type
   * @param type The adapter type
   */
  getFactory(type: string): AdapterFactory | undefined {
    return this.factories.get(type)
  }

  /**
   * Get all registered adapter factories
   */
  getAllFactories(): AdapterFactory[] {
    return Array.from(this.factories.values())
  }

  /**
   * Get all registered adapter types
   */
  getAdapterTypes(): string[] {
    return Array.from(this.factories.keys())
  }

  /**
   * Check if a factory for the given type is registered
   * @param type The adapter type
   */
  hasFactory(type: string): boolean {
    return this.factories.has(type)
  }
}
