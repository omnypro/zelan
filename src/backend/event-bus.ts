/**
 * Event bus implementation for distributing events from adapters to subscribers
 */

import { StreamEvent, EventBusStats } from './types';

// Type for event callback functions
export type EventCallback = (event: StreamEvent) => void;

/**
 * Central event bus for distributing events from adapters to subscribers
 */
export class EventBus {
  private subscribers: Map<string, Set<EventCallback>> = new Map();
  private stats: EventBusStats = {
    events_published: 0,
    events_dropped: 0,
    source_counts: {},
    type_counts: {},
  };
  private buffer_size: number;

  /**
   * Create a new EventBus
   * @param capacity Maximum number of events to buffer
   */
  constructor(capacity: number = 1000) {
    console.log(`Creating event bus with capacity ${capacity}`);
    this.buffer_size = capacity;
    this.subscribers.set('all', new Set());
  }

  /**
   * Subscribe to all events
   * @returns Unsubscribe function
   */
  subscribe(callback: EventCallback): () => void {
    if (!this.subscribers.has('all')) {
      this.subscribers.set('all', new Set());
    }
    
    this.subscribers.get('all')!.add(callback);
    
    // Return unsubscribe function
    return () => {
      const subs = this.subscribers.get('all');
      if (subs) {
        subs.delete(callback);
      }
    };
  }

  /**
   * Subscribe to events from a specific source
   * @param source Source to subscribe to (e.g., "twitch", "obs")
   * @param callback Function to call when events are received
   * @returns Unsubscribe function
   */
  subscribeToSource(source: string, callback: EventCallback): () => void {
    const key = `source:${source}`;
    
    if (!this.subscribers.has(key)) {
      this.subscribers.set(key, new Set());
    }
    
    this.subscribers.get(key)!.add(callback);
    
    // Return unsubscribe function
    return () => {
      const subs = this.subscribers.get(key);
      if (subs) {
        subs.delete(callback);
      }
    };
  }

  /**
   * Subscribe to specific event types
   * @param eventType Event type to subscribe to (e.g., "chat.message")
   * @param callback Function to call when events are received
   * @returns Unsubscribe function
   */
  subscribeToType(eventType: string, callback: EventCallback): () => void {
    const key = `type:${eventType}`;
    
    if (!this.subscribers.has(key)) {
      this.subscribers.set(key, new Set());
    }
    
    this.subscribers.get(key)!.add(callback);
    
    // Return unsubscribe function
    return () => {
      const subs = this.subscribers.get(key);
      if (subs) {
        subs.delete(callback);
      }
    };
  }

  /**
   * Publish an event to all subscribers
   * @param event Event to publish
   * @returns Number of receivers
   */
  async publish(event: StreamEvent): Promise<number> {
    // Cache event details
    const { source, event_type } = event;
    
    console.log(`Publishing event: ${source}.${event_type}`);
    
    // Update stats
    this.stats.events_published++;
    this.stats.source_counts[source] = (this.stats.source_counts[source] || 0) + 1;
    this.stats.type_counts[event_type] = (this.stats.type_counts[event_type] || 0) + 1;
    
    let receiverCount = 0;
    
    // Notify all subscribers
    const allSubs = this.subscribers.get('all');
    if (allSubs) {
      receiverCount += allSubs.size;
      allSubs.forEach(callback => callback(event));
    }
    
    // Notify source-specific subscribers
    const sourceSubs = this.subscribers.get(`source:${source}`);
    if (sourceSubs) {
      receiverCount += sourceSubs.size;
      sourceSubs.forEach(callback => callback(event));
    }
    
    // Notify type-specific subscribers
    const typeSubs = this.subscribers.get(`type:${event_type}`);
    if (typeSubs) {
      receiverCount += typeSubs.size;
      typeSubs.forEach(callback => callback(event));
    }
    
    // If no receivers, increment dropped count
    if (receiverCount === 0) {
      this.stats.events_dropped++;
      console.log(`No receivers for event ${source}.${event_type}, message dropped`);
    }
    
    return receiverCount;
  }

  /**
   * Get current event bus statistics
   * @returns Copy of the current statistics
   */
  async getStats(): Promise<EventBusStats> {
    return { ...this.stats };
  }

  /**
   * Reset all statistics counters
   */
  async resetStats(): Promise<void> {
    console.log('Resetting event bus statistics');
    this.stats = {
      events_published: 0,
      events_dropped: 0,
      source_counts: {},
      type_counts: {},
    };
  }

  /**
   * Get the configured capacity of the event bus
   */
  capacity(): number {
    return this.buffer_size;
  }
}