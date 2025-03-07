import { BehaviorSubject, Observable, map } from 'rxjs';
import { AdapterConfig, AdapterState, ServiceAdapter } from './types';
import { EventBus, EventType, createEvent } from '~/core/events';

/**
 * AdapterManager manages the lifecycle of all adapters in the system
 */
export class AdapterManager {
  private static instance: AdapterManager;
  private adapters: Map<string, ServiceAdapter> = new Map();
  private adaptersSubject: BehaviorSubject<Map<string, ServiceAdapter>> = new BehaviorSubject(new Map());
  private eventBus: EventBus = EventBus.getInstance();
  
  private constructor() {
    // Initialize
  }
  
  /**
   * Get singleton instance of AdapterManager
   */
  public static getInstance(): AdapterManager {
    if (!AdapterManager.instance) {
      AdapterManager.instance = new AdapterManager();
    }
    return AdapterManager.instance;
  }
  
  /**
   * Register an adapter
   */
  public registerAdapter(adapter: ServiceAdapter): void {
    if (this.adapters.has(adapter.adapterId)) {
      throw new Error(`Adapter with ID ${adapter.adapterId} already registered`);
    }
    
    this.adapters.set(adapter.adapterId, adapter);
    this.adaptersSubject.next(new Map(this.adapters));
    
    // Log adapter registration
    console.log(`Registered adapter: ${adapter.displayName} (${adapter.adapterId})`);
  }
  
  /**
   * Unregister an adapter
   */
  public unregisterAdapter(adapterId: string): void {
    const adapter = this.adapters.get(adapterId);
    if (!adapter) {
      return;
    }
    
    // Clean up adapter
    adapter.destroy();
    
    // Remove from map
    this.adapters.delete(adapterId);
    this.adaptersSubject.next(new Map(this.adapters));
    
    // Log adapter unregistration
    console.log(`Unregistered adapter: ${adapter.displayName} (${adapter.adapterId})`);
  }
  
  /**
   * Get an adapter by ID
   */
  public getAdapter<T extends ServiceAdapter>(adapterId: string): T | undefined {
    return this.adapters.get(adapterId) as T | undefined;
  }
  
  /**
   * Get all adapters
   */
  public getAllAdapters(): ServiceAdapter[] {
    return Array.from(this.adapters.values());
  }
  
  /**
   * Get adapters as an observable
   */
  public adapters$(): Observable<ServiceAdapter[]> {
    return this.adaptersSubject.pipe(
      map(adaptersMap => Array.from(adaptersMap.values()))
    );
  }
  
  /**
   * Connect all adapters
   */
  public async connectAll(): Promise<void> {
    const connectPromises = this.getAllAdapters()
      .filter(adapter => adapter.config.enabled)
      .map(adapter => adapter.connect().catch(err => {
        console.error(`Error connecting adapter ${adapter.adapterId}:`, err);
      }));
    
    await Promise.all(connectPromises);
  }
  
  /**
   * Disconnect all adapters
   */
  public async disconnectAll(): Promise<void> {
    const disconnectPromises = this.getAllAdapters()
      .map(adapter => adapter.disconnect().catch(err => {
        console.error(`Error disconnecting adapter ${adapter.adapterId}:`, err);
      }));
    
    await Promise.all(disconnectPromises);
  }
  
  /**
   * Destroy all adapters and clean up
   */
  public destroy(): void {
    for (const adapter of this.adapters.values()) {
      try {
        adapter.destroy();
      } catch (error) {
        console.error(`Error destroying adapter ${adapter.adapterId}:`, error);
      }
    }
    
    this.adapters.clear();
    this.adaptersSubject.next(new Map());
  }
}