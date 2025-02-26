/**
 * Base adapter implementation with common functionality
 */

import { EventBus } from '../event-bus';
import { StreamEvent, BaseAdapterConfig } from '../types';

/**
 * Base interface that all service adapters must implement
 */
export interface ServiceAdapter {
  /**
   * Connect to the service
   */
  connect(): Promise<void>;
  
  /**
   * Disconnect from the service
   */
  disconnect(): Promise<void>;
  
  /**
   * Check if the adapter is currently connected
   */
  isConnected(): boolean;
  
  /**
   * Get the adapter's name
   */
  getName(): string;
  
  /**
   * Configure the adapter
   * @param config Configuration parameters
   */
  configure(config: any): Promise<void>;
}

/**
 * Base adapter implementation with common functionality
 */
export class BaseAdapter {
  private name: string;
  private eventBus: EventBus;
  private connected: boolean = false;
  private eventHandlers: Map<string, NodeJS.Timeout> = new Map();
  
  /**
   * Create a new base adapter
   * @param name Name of the adapter
   * @param eventBus Event bus for publishing events
   */
  constructor(name: string, eventBus: EventBus) {
    console.log(`Creating new adapter: ${name}`);
    this.name = name;
    this.eventBus = eventBus;
  }
  
  /**
   * Get the name of the adapter
   */
  public getName(): string {
    return this.name;
  }
  
  /**
   * Get a reference to the event bus
   */
  public getEventBus(): EventBus {
    return this.eventBus;
  }
  
  /**
   * Check if the adapter is connected
   */
  public isConnected(): boolean {
    return this.connected;
  }
  
  /**
   * Set the connected state
   */
  public setConnected(connected: boolean): void {
    this.connected = connected;
  }
  
  /**
   * Register an interval event handler
   * @param id Unique identifier for the handler
   * @param handler Function to call at interval
   * @param intervalMs Interval in milliseconds
   */
  protected registerIntervalHandler(
    id: string, 
    handler: () => void, 
    intervalMs: number
  ): void {
    // Clear any existing handler with this ID
    this.clearEventHandler(id);
    
    // Create a new interval and store it
    const interval = setInterval(handler, intervalMs);
    this.eventHandlers.set(id, interval);
    
    console.log(`Registered interval handler ${id} with interval ${intervalMs}ms`);
  }
  
  /**
   * Clear an event handler by ID
   * @param id Identifier of the handler to clear
   */
  protected clearEventHandler(id: string): void {
    const handler = this.eventHandlers.get(id);
    if (handler) {
      clearInterval(handler);
      this.eventHandlers.delete(id);
      console.log(`Cleared event handler: ${id}`);
    }
  }
  
  /**
   * Clear all event handlers
   */
  protected clearAllEventHandlers(): void {
    for (const [id, handler] of this.eventHandlers.entries()) {
      clearInterval(handler);
      console.log(`Cleared event handler: ${id}`);
    }
    this.eventHandlers.clear();
  }
  
  /**
   * Publish an event to the event bus
   * @param eventType Type of event (e.g., "test.event")
   * @param payload Event data
   * @returns Number of receivers that received the event
   */
  public async publishEvent(eventType: string, payload: any): Promise<number> {
    console.log(`Publishing ${eventType} event from ${this.name}`);
    
    const event: StreamEvent = {
      source: this.name,
      event_type: eventType,
      payload,
      timestamp: new Date().toISOString()
    };
    
    try {
      const receivers = await this.eventBus.publish(event);
      console.log(`Event published to ${receivers} receivers`);
      return receivers;
    } catch (error) {
      console.error(`Failed to publish event: ${error}`);
      throw error;
    }
  }
}