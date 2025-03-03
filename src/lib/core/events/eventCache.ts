import { BaseEvent } from './types'
import { EventBus } from './eventBus'

/**
 * EventCache - Stores recent events for query access
 */
export class EventCache {
  private static instance: EventCache
  private events: BaseEvent[] = []
  private maxEvents: number = 1000

  private constructor() {
    // Try to load max events setting from config
    try {
      const { ConfigManager } = require('../config/configManager');
      const configManager = ConfigManager.getInstance();
      const eventConfig = configManager.getEventConfig();
      
      if (eventConfig && eventConfig.maxCachedEvents) {
        this.maxEvents = eventConfig.maxCachedEvents;
      }
    } catch (error) {
      // Use default if config isn't available yet
      console.log('Using default event cache size: 1000');
    }
    
    const eventBus = EventBus.getInstance()
    eventBus.events().subscribe((event) => {
      this.addEvent(event)
    })
  }

  /**
   * Get singleton instance of EventCache
   */
  public static getInstance(): EventCache {
    if (!EventCache.instance) {
      EventCache.instance = new EventCache()
    }
    return EventCache.instance
  }

  /**
   * Add an event to the cache
   */
  private addEvent(event: BaseEvent): void {
    // Add to beginning for chronological order (newest first)
    this.events.unshift(event)

    // Trim if over max size
    if (this.events.length > this.maxEvents) {
      this.events = this.events.slice(0, this.maxEvents)
    }
  }

  /**
   * Get recent events from the cache
   */
  public getRecentEvents(count: number = 10): BaseEvent[] {
    return this.events.slice(0, count)
  }

  /**
   * Get recent events of a specific type
   */
  public getRecentEventsByType(type: string, count: number = 10): BaseEvent[] {
    return this.events.filter((event) => event.type === type).slice(0, count)
  }

  /**
   * Get recent events from a specific source
   */
  public getRecentEventsBySource(source: string, count: number = 10): BaseEvent[] {
    return this.events.filter((event) => event.source === source).slice(0, count)
  }

  /**
   * Filter events by type and source
   */
  public filterEvents(options: { type?: string; source?: string; count?: number }): BaseEvent[] {
    const { type, source, count = 10 } = options

    let filtered = this.events

    if (type) {
      filtered = filtered.filter((event) => event.type === type)
    }

    if (source) {
      filtered = filtered.filter((event) => event.source === source)
    }

    return filtered.slice(0, count)
  }

  /**
   * Set maximum cache size
   */
  public setMaxEvents(max: number): void {
    this.maxEvents = max
    // Trim if needed
    if (this.events.length > this.maxEvents) {
      this.events = this.events.slice(0, this.maxEvents)
    }
  }

  /**
   * Clear the event cache
   */
  public clear(): void {
    this.events = []
  }
}
