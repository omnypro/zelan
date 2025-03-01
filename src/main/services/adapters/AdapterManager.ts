import { BehaviorSubject, Observable } from 'rxjs'
import { filter } from 'rxjs/operators'
import { app } from 'electron'
import ElectronStore from 'electron-store'

import { AdapterRegistry, ServiceAdapter, AdapterConfig, AdapterFactory } from '@shared/adapters'
import { mainEventBus } from '@main/services/eventBus'
import { SystemInfoEvent } from '@shared/core/events'

/**
 * Schema for adapter configurations stored in Electron Store
 */
interface AdapterStoreSchema {
  adapters: Record<string, AdapterConfig>
}

/**
 * Service that manages adapters in the main process
 */
export class AdapterManager {
  /**
   * Registry of adapter factories and instances
   */
  private readonly registry = new AdapterRegistry()

  /**
   * Store for persisting adapter configurations
   */
  private readonly store = new ElectronStore<AdapterStoreSchema>({
    name: 'adapters',
    defaults: {
      adapters: {}
    }
  })

  /**
   * Subject that emits when adapters are loaded
   */
  private readonly loadedSubject = new BehaviorSubject<boolean>(false)

  /**
   * Observable that emits when adapters are loaded
   */
  public readonly loaded$: Observable<boolean> = this.loadedSubject.asObservable()

  /**
   * Observable of all adapters
   */
  public readonly adapters$: Observable<ServiceAdapter[]> = this.registry.adapters$

  /**
   * Create a new adapter manager
   */
  constructor() {
    // Log when adapters are loaded
    this.loaded$.pipe(filter((loaded) => loaded)).subscribe(() => {
      mainEventBus.publish(new SystemInfoEvent('Adapters loaded'))
    })

    // Subscribe to app quit event to clean up
    app.on('before-quit', async () => {
      await this.dispose()
    })
  }

  /**
   * Register an adapter factory
   *
   * @param factory Adapter factory to register
   */
  public registerFactory(factory: AdapterFactory): void {
    this.registry.registerFactory(factory)
  }

  /**
   * Register multiple adapter factories
   *
   * @param factories Adapter factories to register
   */
  public registerFactories(factories: AdapterFactory[]): void {
    factories.forEach((factory) => this.registerFactory(factory))
  }

  /**
   * Load adapter configurations from storage and create instances
   */
  public async loadAdapters(): Promise<void> {
    // Get saved adapter configs
    const storedConfigs = this.store.get('adapters', {})

    // Create adapters from stored configs
    for (const id in storedConfigs) {
      const config = storedConfigs[id]

      try {
        // Create the adapter
        await this.registry.createAdapter(config.type, config)
        mainEventBus.publish(new SystemInfoEvent(`Loaded adapter: ${config.name} (${id})`))
      } catch (error) {
        console.error(`Failed to load adapter ${id}:`, error)
        mainEventBus.publish(new SystemInfoEvent(`Failed to load adapter: ${config.name} (${id})`))
      }
    }

    // Notify that adapters are loaded
    this.loadedSubject.next(true)
  }

  /**
   * Create a new adapter instance
   *
   * @param type Type of the adapter to create
   * @param config Configuration for the adapter
   * @returns The created adapter
   */
  public async createAdapter(
    type: string,
    config?: Partial<AdapterConfig>
  ): Promise<ServiceAdapter> {
    // Create the adapter
    const adapter = await this.registry.createAdapter(type, config)

    // Save the configuration
    this.saveAdapterConfig(adapter.id, adapter.getConfig())

    return adapter
  }

  /**
   * Get an adapter by ID
   *
   * @param id ID of the adapter
   * @returns The adapter, or undefined if not found
   */
  public getAdapter(id: string): ServiceAdapter | undefined {
    return this.registry.getAdapter(id)
  }

  /**
   * Get adapters by type
   *
   * @param type Type of adapters to get
   * @returns Array of adapters of the given type
   */
  public getAdaptersByType(type: string): ServiceAdapter[] {
    return this.registry.getAdaptersByType(type)
  }

  /**
   * Observable of adapters by type
   *
   * @param type Type of adapters to observe
   */
  public adaptersByType$(type: string): Observable<ServiceAdapter[]> {
    return this.registry.adaptersByType$(type)
  }

  /**
   * Update an adapter's configuration
   *
   * @param id ID of the adapter
   * @param config New configuration
   */
  public async updateAdapterConfig(id: string, config: Partial<AdapterConfig>): Promise<void> {
    const adapter = this.registry.getAdapter(id)

    if (!adapter) {
      throw new Error(`Adapter not found: ${id}`)
    }

    // Update the adapter
    await adapter.updateConfig(config)

    // Save the updated configuration
    this.saveAdapterConfig(id, adapter.getConfig())
  }

  /**
   * Save an adapter's configuration to storage
   *
   * @param id ID of the adapter
   * @param config Configuration to save
   */
  private saveAdapterConfig(id: string, config: AdapterConfig): void {
    // Get current configs
    const adapters = this.store.get('adapters', {})

    // Update the config
    adapters[id] = config

    // Save to store
    this.store.set('adapters', adapters)
  }

  /**
   * Remove an adapter and its configuration
   *
   * @param id ID of the adapter to remove
   * @returns Whether the adapter was removed
   */
  public async removeAdapter(id: string): Promise<boolean> {
    // Remove from registry
    const result = await this.registry.removeAdapter(id)

    if (result) {
      // Remove from storage
      const adapters = this.store.get('adapters', {})
      delete adapters[id]
      this.store.set('adapters', adapters)
    }

    return result
  }

  /**
   * Clean up resources
   */
  public async dispose(): Promise<void> {
    await this.registry.disposeAll()
  }
}
