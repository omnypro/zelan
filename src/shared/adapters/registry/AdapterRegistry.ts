import { BehaviorSubject, Observable } from 'rxjs'
import { map } from 'rxjs/operators'
import { AdapterFactory, AdapterInfo, ServiceAdapter, AdapterConfig } from '../interfaces'

/**
 * Registry for adapter factories and instances
 */
export class AdapterRegistry {
  /**
   * Map of adapter factories by type
   */
  private factories = new Map<string, AdapterFactory>()

  /**
   * Map of adapter instances by ID
   */
  private adapters = new Map<string, ServiceAdapter>()

  /**
   * Subject that emits changes to the registry
   */
  private readonly changeSubject = new BehaviorSubject<void>(undefined)

  /**
   * Register an adapter factory
   *
   * @param factory Adapter factory to register
   * @throws Error if a factory with the same type is already registered
   */
  public registerFactory(factory: AdapterFactory): void {
    const type = factory.info.type

    if (this.factories.has(type)) {
      throw new Error(`Adapter factory for type '${type}' is already registered`)
    }

    this.factories.set(type, factory)
    this.notifyChange()
  }

  /**
   * Unregister an adapter factory
   *
   * @param type Type of the adapter factory to unregister
   * @returns Whether the factory was unregistered
   */
  public unregisterFactory(type: string): boolean {
    const result = this.factories.delete(type)

    if (result) {
      // Dispose of all adapters of this type
      const adaptersToRemove: string[] = []

      this.adapters.forEach((adapter, id) => {
        if (adapter.type === type) {
          adapter.dispose()
          adaptersToRemove.push(id)
        }
      })

      adaptersToRemove.forEach((id) => {
        this.adapters.delete(id)
      })

      this.notifyChange()
    }

    return result
  }

  /**
   * Get an adapter factory by type
   *
   * @param type Type of the adapter factory
   * @returns The adapter factory, or undefined if not found
   */
  public getFactory(type: string): AdapterFactory | undefined {
    return this.factories.get(type)
  }

  /**
   * Get all registered adapter factories
   *
   * @returns Array of all adapter factories
   */
  public getAllFactories(): AdapterFactory[] {
    return Array.from(this.factories.values())
  }

  /**
   * Get information about all registered adapter types
   *
   * @returns Array of adapter type information
   */
  public getAdapterTypes(): AdapterInfo[] {
    return this.getAllFactories().map((factory) => factory.info)
  }

  /**
   * Observable of adapter type information
   */
  public get adapterTypes$(): Observable<AdapterInfo[]> {
    return this.changeSubject.pipe(map(() => this.getAdapterTypes()))
  }

  /**
   * Create a new adapter instance
   *
   * @param type Type of the adapter to create
   * @param config Configuration for the adapter
   * @returns The created adapter
   * @throws Error if no factory is found for the given type
   */
  public async createAdapter(
    type: string,
    config?: Partial<AdapterConfig>
  ): Promise<ServiceAdapter> {
    const factory = this.factories.get(type)

    if (!factory) {
      throw new Error(`No adapter factory found for type '${type}'`)
    }

    // Get default config and merge with provided config
    const defaultConfig = factory.getDefaultConfig()
    const mergedConfig: AdapterConfig = {
      ...defaultConfig,
      ...config
    }

    // Create the adapter
    const adapter = factory.createAdapter(mergedConfig)

    // Initialize the adapter
    await adapter.initialize(mergedConfig)

    // Store the adapter
    this.adapters.set(adapter.id, adapter)

    this.notifyChange()

    return adapter
  }

  /**
   * Get an adapter by ID
   *
   * @param id ID of the adapter
   * @returns The adapter, or undefined if not found
   */
  public getAdapter(id: string): ServiceAdapter | undefined {
    return this.adapters.get(id)
  }

  /**
   * Get all adapters
   *
   * @returns Array of all adapters
   */
  public getAllAdapters(): ServiceAdapter[] {
    return Array.from(this.adapters.values())
  }

  /**
   * Get adapters by type
   *
   * @param type Type of adapters to get
   * @returns Array of adapters of the given type
   */
  public getAdaptersByType(type: string): ServiceAdapter[] {
    return this.getAllAdapters().filter((adapter) => adapter.type === type)
  }

  /**
   * Observable of all adapters
   */
  public get adapters$(): Observable<ServiceAdapter[]> {
    return this.changeSubject.pipe(map(() => this.getAllAdapters()))
  }

  /**
   * Observable of adapters by type
   *
   * @param type Type of adapters to observe
   */
  public adaptersByType$(type: string): Observable<ServiceAdapter[]> {
    return this.changeSubject.pipe(map(() => this.getAdaptersByType(type)))
  }

  /**
   * Remove an adapter by ID
   *
   * @param id ID of the adapter to remove
   * @returns Whether the adapter was removed
   */
  public async removeAdapter(id: string): Promise<boolean> {
    const adapter = this.adapters.get(id)

    if (!adapter) {
      return false
    }

    // Dispose of the adapter
    await adapter.dispose()

    // Remove from the map
    const result = this.adapters.delete(id)

    if (result) {
      this.notifyChange()
    }

    return result
  }

  /**
   * Dispose of all adapters
   */
  public async disposeAll(): Promise<void> {
    // Dispose of all adapters
    const disposalPromises = this.getAllAdapters().map((adapter) => adapter.dispose())
    await Promise.all(disposalPromises)

    // Clear the adapters map
    this.adapters.clear()

    this.notifyChange()
  }

  /**
   * Notify subscribers about changes to the registry
   */
  private notifyChange(): void {
    this.changeSubject.next(undefined)
  }
}
