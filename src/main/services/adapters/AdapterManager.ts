import { BehaviorSubject, Observable } from 'rxjs'
import { map } from 'rxjs/operators'
import { AdapterRegistry, ServiceAdapter } from '@s/adapters'
import { EventBus } from '@s/core/bus'
import { ConfigStore, AdapterConfig } from '@s/core/config'
import { SystemEventType } from '@s/types/events'
import { createSystemEvent } from '@s/core/events'

/**
 * Manages adapter lifecycle and configuration
 */
export class AdapterManager {
  private adapters = new Map<string, ServiceAdapter>()
  private adaptersSubject = new BehaviorSubject<ServiceAdapter[]>([])

  /**
   * Create a new adapter manager
   */
  constructor(
    private registry: AdapterRegistry,
    private eventBus: EventBus,
    private configStore: ConfigStore
  ) {}

  /**
   * Initialize adapters from saved configurations
   */
  async initialize(): Promise<void> {
    // Load adapter configurations
    const adapterConfigs = this.configStore.getAllAdapters()

    // Create adapter instances from configurations
    const initPromises = Object.values(adapterConfigs).map(async (config) => {
      try {
        await this.createAdapter(config)
      } catch (error) {
        console.error(`Failed to initialize adapter ${config.id}:`, error)
        this.eventBus.publish(
          createSystemEvent(
            SystemEventType.ERROR,
            `Failed to initialize adapter ${config.id}: ${error instanceof Error ? error.message : String(error)}`,
            'error',
            { adapterId: config.id, adapterType: config.type }
          )
        )
      }
    })

    await Promise.all(initPromises)
  }

  /**
   * Create a new adapter from configuration
   */
  async createAdapter(config: AdapterConfig): Promise<ServiceAdapter> {
    const { id, type, name, options } = config

    // Check if adapter with this ID already exists
    if (this.adapters.has(id)) {
      throw new Error(`Adapter with ID ${id} already exists`)
    }

    // Find factory for this adapter type
    const factory = this.registry.getFactory(type)
    if (!factory) {
      throw new Error(`No factory registered for adapter type ${type}`)
    }

    try {
      // Create adapter instance with correct number of arguments
      // Last parameter (enabled) is optional, so we pass only 4 arguments
      const adapter = factory.create(id, name, options || {}, this.eventBus)

      // Initialize adapter
      await adapter.initialize()

      // Store the adapter
      this.adapters.set(id, adapter)
      this.adaptersSubject.next(Array.from(this.adapters.values()))

      // Save configuration
      this.configStore.saveAdapter({
        id,
        type,
        name,
        enabled: adapter.enabled,
        options: adapter.options
      })

      // Publish adapter created event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `Adapter ${name} (${id}) created`, 'info', {
          adapterId: id,
          adapterType: type
        })
      )

      return adapter
    } catch (error) {
      console.error(`Failed to create adapter ${id}:`, error)
      this.eventBus.publish(
        createSystemEvent(
          SystemEventType.ERROR,
          `Failed to create adapter ${name} (${id}): ${error instanceof Error ? error.message : String(error)}`,
          'error',
          { adapterId: id, adapterType: type }
        )
      )
      throw error
    }
  }

  /**
   * Update an existing adapter's configuration
   */
  async updateAdapter(id: string, config: Partial<AdapterConfig>): Promise<ServiceAdapter> {
    const adapter = this.adapters.get(id)
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`)
    }

    try {
      // Update adapter
      await adapter.updateConfig(config)

      // Update state
      this.adaptersSubject.next(Array.from(this.adapters.values()))

      // Save updated configuration
      const currentConfig = this.configStore.getAdapter(id)
      if (currentConfig) {
        this.configStore.saveAdapter({
          ...currentConfig,
          ...config,
          id: currentConfig.id, // ensure ID doesn't change
          type: currentConfig.type // ensure type doesn't change
        })
      }

      // Publish adapter updated event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `Adapter ${adapter.name} (${id}) updated`, 'info', {
          adapterId: id,
          adapterType: adapter.type
        })
      )

      return adapter
    } catch (error) {
      console.error(`Failed to update adapter ${id}:`, error)
      this.eventBus.publish(
        createSystemEvent(
          SystemEventType.ERROR,
          `Failed to update adapter ${adapter.name} (${id}): ${error instanceof Error ? error.message : String(error)}`,
          'error',
          { adapterId: id, adapterType: adapter.type }
        )
      )
      throw error
    }
  }

  /**
   * Start an adapter by ID
   */
  async startAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id)
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`)
    }

    try {
      await adapter.connect()

      // Publish adapter started event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `Adapter ${adapter.name} (${id}) started`, 'info', {
          adapterId: id,
          adapterType: adapter.type
        })
      )
    } catch (error) {
      console.error(`Failed to start adapter ${id}:`, error)
      this.eventBus.publish(
        createSystemEvent(
          SystemEventType.ERROR,
          `Failed to start adapter ${adapter.name} (${id}): ${error instanceof Error ? error.message : String(error)}`,
          'error',
          { adapterId: id, adapterType: adapter.type }
        )
      )
      throw error
    }
  }

  /**
   * Stop an adapter by ID
   */
  async stopAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id)
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`)
    }

    try {
      await adapter.disconnect()

      // Publish adapter stopped event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `Adapter ${adapter.name} (${id}) stopped`, 'info', {
          adapterId: id,
          adapterType: adapter.type
        })
      )
    } catch (error) {
      console.error(`Failed to stop adapter ${id}:`, error)
      this.eventBus.publish(
        createSystemEvent(
          SystemEventType.ERROR,
          `Failed to stop adapter ${adapter.name} (${id}): ${error instanceof Error ? error.message : String(error)}`,
          'error',
          { adapterId: id, adapterType: adapter.type }
        )
      )
      throw error
    }
  }

  /**
   * Delete an adapter by ID
   */
  async deleteAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id)
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`)
    }

    try {
      // Dispose adapter resources
      await adapter.dispose()

      // Remove from collections
      this.adapters.delete(id)
      this.adaptersSubject.next(Array.from(this.adapters.values()))

      // Remove from config
      this.configStore.deleteAdapter(id)

      // Publish adapter deleted event
      this.eventBus.publish(
        createSystemEvent(SystemEventType.INFO, `Adapter ${adapter.name} (${id}) deleted`, 'info', {
          adapterId: id,
          adapterType: adapter.type
        })
      )
    } catch (error) {
      console.error(`Failed to delete adapter ${id}:`, error)
      this.eventBus.publish(
        createSystemEvent(
          SystemEventType.ERROR,
          `Failed to delete adapter ${adapter.name} (${id}): ${error instanceof Error ? error.message : String(error)}`,
          'error',
          { adapterId: id, adapterType: adapter.type }
        )
      )
      throw error
    }
  }

  /**
   * Get available adapter types
   */
  getAvailableAdapterTypes(): string[] {
    return this.registry.getAdapterTypes()
  }

  /**
   * Get an adapter by ID
   */
  getAdapter(id: string): ServiceAdapter | undefined {
    return this.adapters.get(id)
  }

  /**
   * Get all adapters
   */
  getAllAdapters(): ServiceAdapter[] {
    return Array.from(this.adapters.values())
  }

  /**
   * Get adapters of a specific type
   */
  getAdaptersByType(type: string): ServiceAdapter[] {
    return Array.from(this.adapters.values()).filter((adapter) => adapter.type === type)
  }

  /**
   * Observable of all adapters
   */
  adapters$(): Observable<ServiceAdapter[]> {
    return this.adaptersSubject.asObservable()
  }

  /**
   * Observable of adapters by type
   */
  adaptersByType$(type: string): Observable<ServiceAdapter[]> {
    return this.adaptersSubject.pipe(
      map((adapters) => adapters.filter((adapter) => adapter.type === type))
    )
  }

  /**
   * Reconnect all adapters
   */
  async reconnectAll(): Promise<void> {
    const promises: Promise<void>[] = []

    for (const adapter of this.adapters.values()) {
      if (adapter.enabled) {
        promises.push(
          adapter.reconnect().catch((error) => {
            console.error(`Failed to reconnect adapter ${adapter.id}:`, error)
          })
        )
      }
    }

    await Promise.all(promises)
  }

  /**
   * Dispose of all adapters
   */
  async dispose(): Promise<void> {
    const disposePromises = Array.from(this.adapters.values()).map(async (adapter) => {
      try {
        await adapter.dispose()
      } catch (error) {
        console.error(`Failed to dispose adapter ${adapter.id}:`, error)
      }
    })

    await Promise.all(disposePromises)
    this.adapters.clear()
    this.adaptersSubject.next([])
  }
}
