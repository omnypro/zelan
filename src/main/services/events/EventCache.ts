import { BehaviorSubject, Observable } from 'rxjs'
import { map } from 'rxjs/operators'
import { BaseEvent } from '@s/types/events'
import { ConfigStore } from '@s/core/config/ConfigStore'
import { EventFilterCriteria, filterEvents } from '@s/core/bus'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

/**
 * Options for getting events from cache, including filter criteria
 */
export interface EventCacheOptions extends EventFilterCriteria {
  limit?: number
}

/**
 * In-memory cache for recent events
 */
export class EventCache {
  private events: BaseEvent[] = []
  private eventsSubject = new BehaviorSubject<BaseEvent[]>([])
  private cacheSize: number
  private logger: ComponentLogger

  constructor(configStore: ConfigStore) {
    // Initialize logger
    this.logger = getLoggingService().createLogger('EventCache')
    // Set a default cache size
    this.cacheSize = 100

    try {
      // Try to get from settings, but use default if not available
      const settings = configStore.getSettings?.()
      if (settings && typeof settings.eventCacheSize === 'number') {
        this.cacheSize = settings.eventCacheSize
      }
    } catch (error) {
      this.logger.warn('Could not get event cache size from settings, using default', {
        defaultSize: this.cacheSize
      })
    }

    // Listen for settings changes
    try {
      configStore.settings$?.()?.subscribe?.((settings) => {
        if (settings && typeof settings.eventCacheSize === 'number') {
          const newCacheSize = settings.eventCacheSize
          if (this.cacheSize !== newCacheSize) {
            this.cacheSize = newCacheSize
            this.pruneCache()
          }
        }
      })
    } catch (error) {
      this.logger.warn('Could not subscribe to settings changes', {
        error: error instanceof Error ? error.message : String(error)
      })
    }
  }

  addEvent(event: BaseEvent): void {
    // Add to front of array (newest first)
    this.events.unshift(event)

    // Prune if needed
    this.pruneCache()

    // Notify subscribers
    this.eventsSubject.next([...this.events])
  }

  private pruneCache(): void {
    if (this.events.length > this.cacheSize) {
      this.events = this.events.slice(0, this.cacheSize)
    }
  }

  getEvents(options: EventCacheOptions = {}): BaseEvent[] {
    const { limit = 20, ...filterCriteria } = options

    // Apply filters using the filterEvents utility
    const filtered = filterEvents(this.events, filterCriteria)

    // Apply limit
    return filtered.slice(0, limit)
  }

  events$(): Observable<BaseEvent[]> {
    return this.eventsSubject.asObservable()
  }

  filteredEvents$(filterCriteria: EventFilterCriteria = {}): Observable<BaseEvent[]> {
    return this.eventsSubject.pipe(map((events) => filterEvents(events, filterCriteria)))
  }

  clearCache(): void {
    this.events = []
    this.eventsSubject.next([])
  }
}
