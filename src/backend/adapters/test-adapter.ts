/**
 * A simple test adapter that generates events at regular intervals
 * Useful for testing the event bus and WebSocket server without external services
 */

import { BaseAdapter, ServiceAdapter } from './base-adapter';
import { EventBus } from '../event-bus';
import { TestAdapterConfig } from '../types';
import { eventBusInstance } from '../state';

// Singleton instance
let testAdapterInstance: TestAdapter | null = null;

/**
 * Get the test adapter singleton instance
 */
export function getTestAdapter(): TestAdapter {
  if (!testAdapterInstance) {
    testAdapterInstance = new TestAdapter(eventBusInstance);
  }
  return testAdapterInstance;
}

/**
 * Implementation of the TestAdapter that mimics the Rust TestAdapter functionality
 */
export class TestAdapter implements ServiceAdapter {
  private base: BaseAdapter;
  private config: TestAdapterConfig;
  private counter: number = 0;
  
  /**
   * Create a new test adapter
   * @param eventBus EventBus instance
   * @param config Optional configuration
   */
  constructor(eventBus: EventBus, config?: Partial<TestAdapterConfig>) {
    console.log('Creating new test adapter');
    this.base = new BaseAdapter('test', eventBus);
    
    // Apply default config values
    this.config = {
      interval_ms: 1000, // Default: 1 second
      generate_special_events: true,
      ...config
    };
    
    // Clamp interval to reasonable values (100ms to 60s)
    this.config.interval_ms = Math.max(100, Math.min(60000, this.config.interval_ms));
  }
  
  /**
   * Get the adapter name
   */
  getName(): string {
    return this.base.getName();
  }
  
  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.base.isConnected();
  }
  
  /**
   * Configure the test adapter
   * @param config New configuration settings
   */
  async configure(config: Partial<TestAdapterConfig>): Promise<void> {
    console.log('Configuring test adapter', config);
    
    // Merge configs, ensuring interval is within bounds
    this.config = {
      ...this.config,
      ...config
    };
    
    // Clamp interval to reasonable values (100ms to 60s)
    this.config.interval_ms = Math.max(100, Math.min(60000, this.config.interval_ms));
    
    // If connected, restart the event generator with new settings
    if (this.isConnected()) {
      await this.disconnect();
      await this.connect();
    }
  }
  
  /**
   * Connect to the test service (start generating events)
   */
  async connect(): Promise<void> {
    // Only connect if not already connected
    if (this.isConnected()) {
      console.log('Test adapter is already connected');
      return;
    }
    
    console.log('Connecting test adapter');
    
    // Set connected state
    this.base.setConnected(true);
    
    // Start generating events
    this.startEventGenerator();
    
    console.log('Test adapter connected and generating events');
  }
  
  /**
   * Disconnect from the test service (stop generating events)
   */
  async disconnect(): Promise<void> {
    // Only disconnect if connected
    if (!this.isConnected()) {
      console.log('Test adapter is already disconnected');
      return;
    }
    
    console.log('Disconnecting test adapter');
    
    // Set disconnected state
    this.base.setConnected(false);
    
    // Stop event generators
    this.base.clearAllEventHandlers();
    
    console.log('Test adapter disconnected');
  }
  
  /**
   * Get current adapter configuration
   */
  getConfig(): TestAdapterConfig {
    return { ...this.config };
  }
  
  /**
   * Start generating test events at regular intervals
   */
  private startEventGenerator(): void {
    // Register the event handler
    this.base.registerIntervalHandler(
      'test-events',
      () => this.generateEvent(),
      this.config.interval_ms
    );
  }
  
  /**
   * Generate a test event
   */
  private async generateEvent(): Promise<void> {
    // Skip if disconnected
    if (!this.isConnected()) {
      return;
    }
    
    // Create event payload
    const payload = {
      counter: this.counter,
      message: `Test event #${this.counter}`,
      timestamp: new Date().toISOString(),
      source: 'typescript_backend'
    };
    
    console.log(`Generating test event #${this.counter}`);
    
    // Publish regular event
    try {
      await this.base.publishEvent('test.event', payload);
    } catch (error) {
      console.error(`Failed to publish test event: ${error}`);
    }
    
    // Generate special event every 5 counts if enabled
    if (this.config.generate_special_events && this.counter % 5 === 0) {
      const specialPayload = {
        counter: this.counter,
        special: true,
        message: 'This is a special test event',
        timestamp: new Date().toISOString(),
        source: 'typescript_backend'
      };
      
      console.log(`Generating special test event #${this.counter}`);
      
      try {
        await this.base.publishEvent('test.special', specialPayload);
      } catch (error) {
        console.error(`Failed to publish special test event: ${error}`);
      }
    }
    
    // Increment counter
    this.counter++;
  }
}