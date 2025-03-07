import { interval, Subscription, takeUntil } from 'rxjs';
import { BaseAdapter } from './baseAdapter';
import { EventBus, createEvent } from '~/core/events';
import { AdapterSettingsStore } from '~/store';
import {
  TestAdapterConfig,
  TestAdapterConfigSchema,
  BaseEventSchema
} from '@shared/types';

/**
 * Test events to simulate
 */
const TEST_EVENTS = [
  { 
    type: 'test.event.1', 
    data: { message: 'This is test event 1' }
  },
  { 
    type: 'test.event.2', 
    data: { message: 'This is test event 2' }
  },
  { 
    type: 'test.event.3', 
    data: { message: 'This is test event 3', value: 42 }
  },
  { 
    type: 'test.event.4', 
    data: { message: 'This is test event 4', value: 'hello world' }
  },
  { 
    type: 'test.event.5', 
    data: { message: 'This is test event 5', array: [1, 2, 3] }
  },
];

/**
 * Test adapter implementation that generates sample events
 * for testing and development
 */
export class TestAdapter extends BaseAdapter<TestAdapterConfig> {
  private eventTimer: Subscription | null = null;
  private eventCounter = 0;
  
  /**
   * Create a new test adapter instance
   */
  constructor(config: Partial<TestAdapterConfig> = {}) {
    // Try to get settings from AdapterSettingsStore
    let mergedConfig = config;
    
    try {
      const adapterSettingsStore = AdapterSettingsStore.getInstance();
      const savedSettings = adapterSettingsStore.getSettings('test-adapter');
      
      if (savedSettings) {
        // Merge saved settings with provided config, with provided config taking precedence
        mergedConfig = {
          ...savedSettings,
          ...config
        };
        console.log('Test adapter loaded settings from AdapterSettingsStore');
      }
    } catch (error) {
      console.warn('Could not load Test adapter settings:', error);
    }
    
    super('test-adapter', 'Test Adapter', TestAdapterConfigSchema, mergedConfig);
  }
  
  /**
   * Implementation of connect
   */
  protected async connectImpl(): Promise<void> {
    // Simulate a connection delay
    await new Promise(resolve => setTimeout(resolve, 500));
    
    // Start generating events
    this.startEventTimer();
  }
  
  /**
   * Implementation of disconnect
   */
  protected async disconnectImpl(): Promise<void> {
    // Stop generating events
    this.stopEventTimer();
  }
  
  /**
   * Start generating events
   */
  private startEventTimer(): void {
    // Stop any existing timer
    this.stopEventTimer();
    
    // Start new timer
    const { interval: eventInterval } = this.config;
    
    this.eventTimer = interval(eventInterval)
      .pipe(takeUntil(this.destroy$))
      .subscribe(() => {
        this.generateEvent();
      });
  }
  
  /**
   * Stop generating events
   */
  private stopEventTimer(): void {
    if (this.eventTimer) {
      this.eventTimer.unsubscribe();
      this.eventTimer = null;
    }
  }
  
  /**
   * Generate a random test event
   */
  private generateEvent(): void {
    const eventBus = EventBus.getInstance();
    this.eventCounter++;
    
    // Randomly generate an error event if enabled
    if (this.config.generateErrors && Math.random() < 0.1) {
      this.generateErrorEvent();
      return;
    }
    
    // Select a random event type
    const eventIndex = Math.floor(Math.random() * TEST_EVENTS.length);
    const eventTemplate = TEST_EVENTS[eventIndex];
    
    // Publish the event
    eventBus.publish(createEvent(
      BaseEventSchema,
      {
        type: eventTemplate.type,
        source: 'test-adapter',
        adapterId: this.adapterId,
        data: {
          ...eventTemplate.data,
          counter: this.eventCounter,
          timestamp: Date.now(),
        }
      }
    ));
  }
  
  /**
   * Generate a random error event
   */
  private generateErrorEvent(): void {
    console.log('Test adapter generating error event');
    
    const error = new Error(`Test error #${this.eventCounter}`);
    this.publishErrorEvent(error);
  }
}