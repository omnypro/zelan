import { BehaviorSubject, Observable, Subject } from 'rxjs'
import { map, debounceTime, takeUntil, distinctUntilChanged } from 'rxjs/operators'
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
 * In-memory cache for recent events with improved performance
 */
export class EventCache {
  private events: BaseEvent[] = []
  private eventsSubject = new BehaviorSubject<BaseEvent[]>([])
  private batchUpdates = new Subject<BaseEvent>()
  private cacheSize: number
  private logger: ComponentLogger
  private destroy$ = new Subject<void>()

  constructor(configStore: ConfigStore) {
    // Initialize logger
    this.logger = getLoggingService().createLogger('EventCache')

    // Set a default cache size
    this.cacheSize = 100

    try {
      // Try to get from settings, but use default if not available
      const settings = configStore.getSettings()
      if (settings && typeof settings.eventCacheSize === 'number') {
        this.cacheSize = settings.eventCacheSize
      }
    } catch (error) {
      this.logger.warn('Could not get event cache size from settings, using default', {
        defaultSize: this.cacheSize
      })
    }

    // Set up batch processing for efficient updates
    this.setupBatchProcessing()

    // Listen for settings changes
    try {
      configStore
        .settings$()
        .pipe(
          takeUntil(this.destroy$),
          map((settings) => settings?.eventCacheSize),
          distinctUntilChanged()
        )
        .subscribe((newCacheSize) => {
          if (typeof newCacheSize === 'number' && this.cacheSize !== newCacheSize) {
            this.logger.debug('Updating event cache size', {
              oldSize: this.cacheSize,
              newSize: newCacheSize
            })
            this.cacheSize = newCacheSize
            this.pruneCache()
          }
        })
    } catch (error) {
      this.logger.warn('Could not subscribe to settings changes', {
        error: error instanceof Error ? error.message : String(error)
      })
    }
  }

  /**
   * Set up batch processing of events to improve performance
   */
  private setupBatchProcessing(): void {
    // Process events in batches with a small delay to reduce updates
    this.batchUpdates
      .pipe(
        takeUntil(this.destroy$),
        debounceTime(50) // Process updates in 50ms batches
      )
      .subscribe(() => {
        // Notify subscribers with a fresh copy to maintain immutability
        this.eventsSubject.next([...this.events])
      })
  }

  /**
   * Add an event to the cache
   */
  addEvent(event: BaseEvent): void {
    // Add to front of array (newest first)
    this.events.unshift(event)

    // Prune if needed
    if (this.events.length > this.cacheSize) {
      this.pruneCache()
    }

    // Queue update notification (will be batched)
    this.batchUpdates.next(event)
  }

  /**
   * Trim the cache to the configured size
   */
  private pruneCache(): void {
    if (this.events.length > this.cacheSize) {
      this.events = this.events.slice(0, this.cacheSize)
      // Force an update after pruning
      this.eventsSubject.next([...this.events])
    }
  }

  /**
   * Get events from the cache with filtering and limits
   */
  getEvents(options: EventCacheOptions = {}): BaseEvent[] {
    const { limit = 20, ...filterCriteria } = options

    // Apply filters using the filterEvents utility
    const filtered = filterEvents(this.events, filterCriteria)

    // Apply limit and return
    return filtered.slice(0, limit)
  }

  /**
   * Observable of all events in the cache
   */
  events$(): Observable<BaseEvent[]> {
    return this.eventsSubject.asObservable()
  }

  /**
   * Observable of filtered events from the cache
   */
  filteredEvents$(filterCriteria: EventFilterCriteria = {}, limit?: number): Observable<BaseEvent[]> {
    return this.eventsSubject.pipe(
      map((events) => {
        // Apply filter
        const filtered = filterEvents(events, filterCriteria)
        // Apply limit if provided
        return limit ? filtered.slice(0, limit) : filtered
      })
    )
  }

  /**
   * Clear all events from the cache
   */
  clearCache(): void {
    this.events = []
    this.eventsSubject.next([])
  }

  /**
   * Clean up resources
   */
  dispose(): void {
    this.destroy$.next()
    this.destroy$.complete()
    this.events = []
  }
}
