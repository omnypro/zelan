import { BehaviorSubject, Observable, Subject } from 'rxjs';
import { z } from 'zod';
import { EventBus, EventType, createEvent, BaseEventSchema } from '../events';
import { AdapterConfig, AdapterConfigSchema, AdapterState, ServiceAdapter } from './types';

/**
 * Adapter event schema
 */
export const AdapterEventSchema = BaseEventSchema.extend({
  adapterId: z.string(),
  state: z.nativeEnum(AdapterState),
  error: z.string().optional(),
});

export type AdapterEvent = z.infer<typeof AdapterEventSchema>;

/**
 * Base adapter implementation that handles common adapter functionality
 */
export abstract class BaseAdapter<T extends AdapterConfig = AdapterConfig> implements ServiceAdapter<T> {
  private eventBus: EventBus = EventBus.getInstance();
  private stateSubject: BehaviorSubject<AdapterState>;
  private destroy$ = new Subject<void>();
  private configValue: T;
  private configSchema: z.ZodType<T>;
  
  /**
   * Create a new adapter instance
   */
  constructor(
    public readonly adapterId: string,
    public readonly displayName: string,
    config: Partial<T>,
    configSchema?: z.ZodType<T>
  ) {
    // Use provided schema or default to AdapterConfigSchema
    this.configSchema = (configSchema || AdapterConfigSchema) as z.ZodType<T>;
    
    // Initialize state
    this.stateSubject = new BehaviorSubject<AdapterState>(AdapterState.DISCONNECTED);
    
    // Initialize config with defaults
    this.configValue = this.configSchema.parse(config);
    
    // Auto-connect if enabled
    if (this.config.autoConnect && this.config.enabled) {
      this.connect().catch(err => {
        console.error(`Failed to auto-connect adapter ${this.adapterId}:`, err);
      });
    }
  }
  
  /**
   * Get current connection state
   */
  public get state(): AdapterState {
    return this.stateSubject.getValue();
  }
  
  /**
   * Get connection state as an observable
   */
  public get state$(): Observable<AdapterState> {
    return this.stateSubject.asObservable();
  }
  
  /**
   * Get current configuration
   */
  public get config(): T {
    return this.configValue;
  }
  
  /**
   * Connect to the service
   */
  public async connect(): Promise<void> {
    // Only connect if disconnected and enabled
    if (this.state !== AdapterState.DISCONNECTED || !this.config.enabled) {
      return;
    }
    
    // Update state to connecting
    this.updateState(AdapterState.CONNECTING);
    
    try {
      // Call the concrete implementation
      await this.connectImpl();
      
      // Update state to connected
      this.updateState(AdapterState.CONNECTED);
      
      // Publish connection event
      this.eventBus.publish(createEvent(
        AdapterEventSchema,
        {
          type: EventType.ADAPTER_CONNECTED,
          source: this.adapterId,
          adapterId: this.adapterId,
          state: AdapterState.CONNECTED,
        }
      ));
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Update state to error
      this.updateState(AdapterState.ERROR);
      
      // Then back to disconnected after a delay
      setTimeout(() => this.updateState(AdapterState.DISCONNECTED), 1000);
      
      // Publish error event
      this.eventBus.publish(createEvent(
        AdapterEventSchema,
        {
          type: EventType.ADAPTER_ERROR,
          source: this.adapterId,
          adapterId: this.adapterId,
          state: AdapterState.ERROR,
          error: errorMessage,
        }
      ));
      
      throw new Error(`Failed to connect to ${this.displayName}: ${errorMessage}`);
    }
  }
  
  /**
   * Disconnect from the service
   */
  public async disconnect(): Promise<void> {
    // Only disconnect if connected
    if (this.state !== AdapterState.CONNECTED) {
      return;
    }
    
    try {
      // Call the concrete implementation
      await this.disconnectImpl();
      
      // Update state to disconnected
      this.updateState(AdapterState.DISCONNECTED);
      
      // Publish disconnection event
      this.eventBus.publish(createEvent(
        AdapterEventSchema,
        {
          type: EventType.ADAPTER_DISCONNECTED,
          source: this.adapterId,
          adapterId: this.adapterId,
          state: AdapterState.DISCONNECTED,
        }
      ));
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      
      // Update state to error
      this.updateState(AdapterState.ERROR);
      
      // Then back to disconnected after a delay
      setTimeout(() => this.updateState(AdapterState.DISCONNECTED), 1000);
      
      // Publish error event
      this.eventBus.publish(createEvent(
        AdapterEventSchema,
        {
          type: EventType.ADAPTER_ERROR,
          source: this.adapterId,
          adapterId: this.adapterId,
          state: AdapterState.ERROR,
          error: errorMessage,
        }
      ));
      
      throw new Error(`Failed to disconnect from ${this.displayName}: ${errorMessage}`);
    }
  }
  
  /**
   * Update adapter configuration
   */
  public updateConfig(config: Partial<T>): void {
    try {
      // Merge with current config
      const newConfig = {
        ...this.configValue,
        ...config,
      };
      
      // Validate new config
      this.configValue = this.configSchema.parse(newConfig);
      
      // Try to save to the persistence layer
      try {
        const { AdapterSettingsManager } = require('../config/adapterSettingsManager');
        const settingsManager = AdapterSettingsManager.getInstance();
        settingsManager.updateSettings(this.adapterId, config);
      } catch (saveError) {
        console.warn(`Could not save adapter settings: ${saveError instanceof Error ? saveError.message : String(saveError)}`);
      }
      
      // Reconnect if necessary
      this.handleConfigChange();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error(`Invalid configuration for adapter ${this.adapterId}:`, errorMessage);
      throw new Error(`Invalid configuration: ${errorMessage}`);
    }
  }
  
  /**
   * Check if adapter is currently connected
   */
  public isConnected(): boolean {
    return this.state === AdapterState.CONNECTED;
  }
  
  /**
   * Destroy the adapter and clean up resources
   */
  public destroy(): void {
    // Disconnect if connected
    if (this.isConnected()) {
      this.disconnect().catch(err => {
        console.error(`Error disconnecting adapter ${this.adapterId}:`, err);
      });
    }
    
    // Complete destroy subject
    this.destroy$.next();
    this.destroy$.complete();
    
    // Call concrete implementation
    this.destroyImpl();
  }
  
  /**
   * Update the adapter state
   */
  protected updateState(state: AdapterState): void {
    this.stateSubject.next(state);
  }
  
  /**
   * Handle configuration changes
   */
  protected handleConfigChange(): void {
    // Default implementation - concrete adapters can override
    if (this.isConnected() && !this.config.enabled) {
      // Disconnect if disabled
      this.disconnect().catch(err => {
        console.error(`Error disconnecting adapter ${this.adapterId}:`, err);
      });
    } else if (!this.isConnected() && this.config.enabled && this.config.autoConnect) {
      // Connect if enabled and auto-connect
      this.connect().catch(err => {
        console.error(`Error connecting adapter ${this.adapterId}:`, err);
      });
    }
  }
  
  /**
   * Get the destroy observable
   */
  protected get destroyed$(): Observable<void> {
    return this.destroy$.asObservable();
  }
  
  /**
   * Concrete implementation of connect
   */
  protected abstract connectImpl(): Promise<void>;
  
  /**
   * Concrete implementation of disconnect
   */
  protected abstract disconnectImpl(): Promise<void>;
  
  /**
   * Concrete implementation of destroy
   */
  protected abstract destroyImpl(): void;
}