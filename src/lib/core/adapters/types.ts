import { BehaviorSubject, Observable } from 'rxjs';
import { z } from 'zod';
import { BaseEvent } from '../events';

/**
 * Adapter connection states
 */
export enum AdapterState {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  ERROR = 'error',
}

/**
 * Schema for adapter configuration
 */
export const AdapterConfigSchema = z.object({
  enabled: z.boolean().default(true),
  name: z.string().optional(),
  autoConnect: z.boolean().default(true),
});

export type AdapterConfig = z.infer<typeof AdapterConfigSchema>;

/**
 * Base service adapter interface that all adapters must implement
 */
export interface ServiceAdapter<T extends AdapterConfig = AdapterConfig> {
  /**
   * Unique identifier for the adapter
   */
  readonly adapterId: string;
  
  /**
   * User-friendly name for the adapter
   */
  readonly displayName: string;
  
  /**
   * Current connection state
   */
  readonly state: AdapterState;
  
  /**
   * Connection state as an observable
   */
  readonly state$: Observable<AdapterState>;
  
  /**
   * Current adapter configuration
   */
  readonly config: T;
  
  /**
   * Connect to the service
   */
  connect(): Promise<void>;
  
  /**
   * Disconnect from the service
   */
  disconnect(): Promise<void>;
  
  /**
   * Update adapter configuration
   */
  updateConfig(config: Partial<T>): void;
  
  /**
   * Check if adapter is currently connected
   */
  isConnected(): boolean;
  
  /**
   * Destroy the adapter and clean up resources
   */
  destroy(): void;
}