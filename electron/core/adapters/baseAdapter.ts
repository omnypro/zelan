import { BehaviorSubject, Observable, Subject } from 'rxjs';
import { z } from 'zod';
import { EventBus, createEvent } from '~/core/events';
import { 
  AdapterConfig, 
  AdapterState, 
  ServiceAdapter, 
  EventType,
  BaseEventSchema
} from '@shared/types';

/**
 * Base adapter implementation that handles common adapter functionality
 */
export abstract class BaseAdapter<T extends AdapterConfig = AdapterConfig> implements ServiceAdapter<T> {
  private stateSubject: BehaviorSubject<AdapterState>;
  protected destroy$ = new Subject<void>();
  private configValue: T;
  private configSchema: z.ZodType<T>;
  
  /**
   * Create a new adapter instance
   */
  constructor(
    public readonly adapterId: string,
    public readonly displayName: string,
    configSchema: z.ZodType<T>,
    config: Partial<T>
  ) {
    // Use provided schema
    this.configSchema = configSchema;
    
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
   * This is the public API that wraps the concrete implementation
   */
  public async connect(): Promise<void> {
    if (this.isConnected()) {
      console.log(`${this.adapterId} is already connected`);
      return;
    }
    
    this.setState(AdapterState.CONNECTING);
    
    try {
      await this.connectImpl();
      this.setState(AdapterState.CONNECTED);
      this.publishConnectedEvent();
    } catch (error) {
      console.error(`Error connecting ${this.adapterId}:`, error);
      this.setState(AdapterState.ERROR);
      this.publishErrorEvent(error);
    }
  }
  
  /**
   * Implementation specific connect logic
   */
  protected abstract connectImpl(): Promise<void>;
  
  /**
   * Disconnect from the service
   * This is the public API that wraps the concrete implementation
   */
  public async disconnect(): Promise<void> {
    if (!this.isConnected()) {
      console.log(`${this.adapterId} is already disconnected`);
      return;
    }
    
    try {
      await this.disconnectImpl();
      this.setState(AdapterState.DISCONNECTED);
      this.publishDisconnectedEvent();
    } catch (error) {
      console.error(`Error disconnecting ${this.adapterId}:`, error);
      this.setState(AdapterState.ERROR);
      this.publishErrorEvent(error);
    }
  }
  
  /**
   * Implementation specific disconnect logic
   */
  protected abstract disconnectImpl(): Promise<void>;
  
  /**
   * Update adapter configuration
   */
  public updateConfig(configUpdate: Partial<T>): void {
    try {
      // Merge with existing config
      const updatedConfig = {
        ...this.configValue,
        ...configUpdate,
      };
      
      // Validate
      this.configValue = this.configSchema.parse(updatedConfig);
      
      console.log(`${this.adapterId} config updated:`, this.configValue);
    } catch (error) {
      console.error(`Error updating ${this.adapterId} config:`, error);
      throw new Error(`Failed to update ${this.adapterId} config: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  
  /**
   * Check if adapter is connected
   */
  public isConnected(): boolean {
    return this.state === AdapterState.CONNECTED;
  }
  
  /**
   * Clean up adapter resources
   */
  public destroy(): void {
    // Disconnect if connected
    if (this.isConnected()) {
      this.disconnect().catch(err => {
        console.error(`Error disconnecting ${this.adapterId} during destroy:`, err);
      });
    }
    
    // Complete destroy$ subject to notify subscribers
    this.destroy$.next();
    this.destroy$.complete();
  }
  
  /**
   * Set adapter state
   */
  protected setState(state: AdapterState): void {
    if (this.state !== state) {
      this.stateSubject.next(state);
    }
  }
  
  /**
   * Publish connected event
   */
  protected publishConnectedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.ADAPTER_CONNECTED,
        source: this.adapterId,
        adapterId: this.adapterId,
        data: {
          displayName: this.displayName
        }
      }
    ));
  }
  
  /**
   * Publish disconnected event
   */
  protected publishDisconnectedEvent(): void {
    const eventBus = EventBus.getInstance();
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.ADAPTER_DISCONNECTED,
        source: this.adapterId,
        adapterId: this.adapterId,
        data: {
          displayName: this.displayName
        }
      }
    ));
  }
  
  /**
   * Publish error event
   */
  protected publishErrorEvent(error: unknown): void {
    const eventBus = EventBus.getInstance();
    const errorMessage = error instanceof Error ? error.message : String(error);
    
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: EventType.ADAPTER_ERROR,
        source: this.adapterId,
        adapterId: this.adapterId,
        data: {
          error: errorMessage,
          displayName: this.displayName
        }
      }
    ));
  }
}