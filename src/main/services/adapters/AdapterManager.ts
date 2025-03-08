import { BehaviorSubject, Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { AdapterRegistry, ServiceAdapter } from '../../../shared/adapters';
import { EventBus } from '../../../shared/core/bus';
import { AdapterConfig, ReactiveConfigStore } from '../../../shared/core/config';

/**
 * Manages adapter lifecycle and configuration
 */
export class AdapterManager {
  private adapters = new Map<string, ServiceAdapter>();
  private adaptersSubject = new BehaviorSubject<ServiceAdapter[]>([]);
  
  constructor(
    private registry: AdapterRegistry,
    private eventBus: EventBus,
    private configStore: ReactiveConfigStore
  ) {}
  
  /**
   * Initialize adapters from saved configurations
   */
  async initialize(): Promise<void> {
    // Load adapter configurations
    const adapterConfigs = this.configStore.get<Record<string, AdapterConfig>>('adapters', {});
    
    // Create adapter instances from configurations
    const initPromises = Object.values(adapterConfigs).map(async (config) => {
      try {
        await this.createAdapter(config);
      } catch (error) {
        console.error(`Failed to initialize adapter ${config.id}:`, error);
      }
    });
    
    await Promise.all(initPromises);
  }
  
  /**
   * Create a new adapter from configuration
   */
  async createAdapter(config: AdapterConfig): Promise<ServiceAdapter> {
    const { id, type, name, options } = config;
    
    // Check if adapter with this ID already exists
    if (this.adapters.has(id)) {
      throw new Error(`Adapter with ID ${id} already exists`);
    }
    
    // Find factory for this adapter type
    const factory = this.registry.getFactory(type);
    if (!factory) {
      throw new Error(`No factory registered for adapter type ${type}`);
    }
    
    // Create adapter instance
    const adapter = factory.create(id, name, options, this.eventBus);
    
    // Initialize adapter
    await adapter.initialize();
    
    // Store the adapter
    this.adapters.set(id, adapter);
    this.adaptersSubject.next(Array.from(this.adapters.values()));
    
    // Save configuration
    this.saveAdapterConfig(adapter);
    
    return adapter;
  }
  
  /**
   * Update an existing adapter's configuration
   */
  async updateAdapter(id: string, config: Partial<AdapterConfig>): Promise<ServiceAdapter> {
    const adapter = this.adapters.get(id);
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`);
    }
    
    // Update adapter
    await adapter.updateConfig(config);
    
    // Save updated configuration
    this.saveAdapterConfig(adapter);
    
    // Notify observers
    this.adaptersSubject.next(Array.from(this.adapters.values()));
    
    return adapter;
  }
  
  /**
   * Remove an adapter
   */
  async removeAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id);
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`);
    }
    
    // Dispose adapter resources
    await adapter.dispose();
    
    // Remove from map
    this.adapters.delete(id);
    
    // Update state
    this.adaptersSubject.next(Array.from(this.adapters.values()));
    
    // Remove from config
    const adapterConfigs = this.configStore.get<Record<string, AdapterConfig>>('adapters', {});
    delete adapterConfigs[id];
    this.configStore.set('adapters', adapterConfigs);
  }
  
  /**
   * Start an adapter by ID
   */
  async startAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id);
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`);
    }
    
    await adapter.connect();
  }
  
  /**
   * Stop an adapter by ID
   */
  async stopAdapter(id: string): Promise<void> {
    const adapter = this.adapters.get(id);
    if (!adapter) {
      throw new Error(`Adapter with ID ${id} not found`);
    }
    
    await adapter.disconnect();
  }
  
  /**
   * Delete an adapter by ID
   */
  async deleteAdapter(id: string): Promise<void> {
    await this.removeAdapter(id);
  }
  
  /**
   * Get available adapter types
   */
  getAvailableAdapterTypes(): string[] {
    return this.registry.getAdapterTypes();
  }
  
  /**
   * Get an adapter by ID
   */
  getAdapter(id: string): ServiceAdapter | undefined {
    return this.adapters.get(id);
  }
  
  /**
   * Get all adapters
   */
  getAllAdapters(): ServiceAdapter[] {
    return Array.from(this.adapters.values());
  }
  
  /**
   * Get adapters of a specific type
   */
  getAdaptersByType(type: string): ServiceAdapter[] {
    return Array.from(this.adapters.values()).filter(adapter => adapter.type === type);
  }
  
  /**
   * Observable of all adapters
   */
  adapters$(): Observable<ServiceAdapter[]> {
    return this.adaptersSubject.asObservable();
  }
  
  /**
   * Observable of adapters by type
   */
  adaptersByType$(type: string): Observable<ServiceAdapter[]> {
    return this.adaptersSubject.pipe(
      map(adapters => adapters.filter(adapter => adapter.type === type))
    );
  }
  
  /**
   * Dispose of all adapters
   */
  async dispose(): Promise<void> {
    const disposePromises = Array.from(this.adapters.values()).map(async (adapter) => {
      try {
        await adapter.dispose();
      } catch (error) {
        console.error(`Failed to dispose adapter ${adapter.id}:`, error);
      }
    });
    
    await Promise.all(disposePromises);
    this.adapters.clear();
    this.adaptersSubject.next([]);
  }
  
  /**
   * Save adapter configuration to store
   */
  private saveAdapterConfig(adapter: ServiceAdapter): void {
    const adapterConfigs = this.configStore.get<Record<string, AdapterConfig>>('adapters', {});
    
    adapterConfigs[adapter.id] = {
      id: adapter.id,
      type: adapter.type,
      name: adapter.name,
      enabled: adapter.enabled,
      options: adapter.options
    };
    
    this.configStore.set('adapters', adapterConfigs);
  }
}