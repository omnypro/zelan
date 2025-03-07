import { interval, Subscription, takeUntil } from 'rxjs';
import { z } from 'zod';
import { BaseAdapter } from './baseAdapter';
import { EventBus, createEvent, BaseEventSchema } from '../events';

/**
 * Test event schemas for generating sample events
 */
export const TestEventSchema = BaseEventSchema.extend({
  testId: z.string(),
  value: z.number(),
  message: z.string(),
});

export type TestEvent = z.infer<typeof TestEventSchema>;

/**
 * Test adapter configuration schema
 */
export const TestAdapterConfigSchema = z.object({
  enabled: z.boolean().default(true),
  name: z.string().optional(),
  autoConnect: z.boolean().default(true),
  interval: z.number().min(100).default(2000),
  generateErrors: z.boolean().default(false),
});

export type TestAdapterConfig = z.infer<typeof TestAdapterConfigSchema>;

/**
 * TestAdapter is a development adapter that generates test events
 * This is useful for testing the event system without actual services
 */
export class TestAdapter extends BaseAdapter<TestAdapterConfig> {
  // Use the eventBus from BaseAdapter instead of creating a new one
  // private eventBus: EventBus = EventBus.getInstance();
  private eventSubscription: Subscription | null = null;
  private eventCount = 0;
  
  /**
   * Create a new test adapter
   */
  constructor(config: Partial<TestAdapterConfig> = {}) {
    // Try to get settings from AdapterSettingsManager
    let mergedConfig = config;
    
    try {
      const { AdapterSettingsManager } = require('../config/adapterSettingsManager');
      const settingsManager = AdapterSettingsManager.getInstance();
      const savedSettings = settingsManager.getSettings<TestAdapterConfig>('test-adapter');
      
      if (savedSettings) {
        // Merge saved settings with provided config, with provided config taking precedence
        mergedConfig = {
          ...savedSettings,
          ...config
        };
        console.log('Test adapter loaded settings from AdapterSettingsManager');
      }
    } catch (error) {
      console.log('Using default test adapter settings');
    }
    
    super(
      'test-adapter',
      'Test Adapter',
      mergedConfig,
      TestAdapterConfigSchema
    );
  }
  
  /**
   * Connect and start generating events
   */
  protected async connectImpl(): Promise<void> {
    // Start generating events
    this.startEventGeneration();
    
    // Simulate connection delay
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  
  /**
   * Disconnect and stop generating events
   */
  protected async disconnectImpl(): Promise<void> {
    // Stop generating events
    this.stopEventGeneration();
    
    // Simulate disconnection delay
    await new Promise(resolve => setTimeout(resolve, 300));
  }
  
  /**
   * Clean up resources
   */
  protected destroyImpl(): void {
    this.stopEventGeneration();
  }
  
  /**
   * Start generating test events at the configured interval
   */
  private startEventGeneration(): void {
    // Stop any existing event generation
    this.stopEventGeneration();
    
    // Create new subscription
    this.eventSubscription = interval(this.config.interval)
      .pipe(takeUntil(this.destroyed$))
      .subscribe(() => {
        // Generate a test event
        this.generateTestEvent();
        
        // Optionally generate an error
        if (this.config.generateErrors && this.eventCount % 10 === 0) {
          this.generateErrorEvent();
        }
        
        this.eventCount++;
      });
  }
  
  /**
   * Stop generating test events
   */
  private stopEventGeneration(): void {
    if (this.eventSubscription) {
      this.eventSubscription.unsubscribe();
      this.eventSubscription = null;
    }
  }
  
  /**
   * Handle configuration changes
   */
  protected override handleConfigChange(): void {
    // Update event generation if connected and interval changed
    if (this.isConnected() && this.eventSubscription) {
      this.startEventGeneration();
    }
    
    // Call the parent implementation for connect/disconnect handling
    super.handleConfigChange();
  }
  
  /**
   * Generate a test event
   */
  private generateTestEvent(): void {
    // Get eventBus from parent BaseAdapter class
    const eventBus = EventBus.getInstance();
    
    const testEvent = createEvent(
      TestEventSchema,
      {
        type: 'test.event',
        source: this.adapterId,
        data: {
          testId: `test-${this.eventCount}`,
          value: Math.floor(Math.random() * 100),
          message: `Test event ${this.eventCount}`
        }
      }
    );
    
    eventBus.publish(testEvent);
  }
  
  /**
   * Generate an error event
   */
  private generateErrorEvent(): void {
    // Get eventBus from parent BaseAdapter class
    const eventBus = EventBus.getInstance();
    
    const errorEvent = createEvent(
      BaseEventSchema,
      {
        type: 'test.error',
        source: this.adapterId,
      }
    );
    
    eventBus.publish(errorEvent);
  }
}