import { Subject, Observable } from 'rxjs'
import { share, filter } from 'rxjs/operators'
import { BrowserWindow, WebContents } from 'electron'
import { BaseEvent, EventCategory, SystemEventType } from '@s/types/events'
import { createSystemEvent } from '@s/core/events'
import { EventBus, EventFilterCriteria } from '@s/core/bus'
import { EventCache, EventCacheOptions } from '../events/EventCache'

/**
 * Main process implementation of the EventBus
 */
export class MainEventBus implements EventBus {
  private eventSubject = new Subject<BaseEvent>()
  private eventCache: EventCache
  private rendererWindows = new Set<WebContents>()

  /**
   * Create the main event bus with event cache
   */
  constructor(eventCache: EventCache) {
    this.eventCache = eventCache

    // All events are multicasted to multiple subscribers
    this.events$ = this.eventSubject.asObservable().pipe(share())

    // Cache all events for short-term access
    this.events$.subscribe((event) => {
      this.eventCache.addEvent(event)
    })
  }

  /**
   * Observable of all events
   */
  readonly events$: Observable<BaseEvent>

  /**
   * Add a new window to receive events
   */
  addWebContents(webContents: WebContents): void {
    if (!this.rendererWindows.has(webContents)) {
      this.rendererWindows.add(webContents)

      // Remove when destroyed
      webContents.once('destroyed', () => {
        this.rendererWindows.delete(webContents)
      })

      // Send welcome event
      this.publish(
        createSystemEvent(SystemEventType.INFO, 'Renderer process connected to event bus', 'info', {
          windowId: webContents.id
        })
      )
    }
  }

  /**
   * Add a browser window to receive events
   */
  addWindow(window: BrowserWindow): void {
    this.addWebContents(window.webContents)
  }

  /**
   * Get events filtered by specified criteria
   */
  getFilteredEvents$<T = unknown>(criteria: EventFilterCriteria<T>): Observable<BaseEvent<T>> {
    // Using type assertion to bridge the gap
    const stream$ = this.events$ as Observable<BaseEvent<T>>

    return stream$.pipe(
      filter((event) => {
        // Skip filtering if no criteria provided
        if (!criteria || Object.keys(criteria).length === 0) return true

        // Check category
        if (criteria.category !== undefined && event.category !== criteria.category) {
          return false
        }

        // Check type
        if (criteria.type !== undefined && event.type !== criteria.type) {
          return false
        }

        // Check sourceId
        if (criteria.sourceId !== undefined && event.source?.id !== criteria.sourceId) {
          return false
        }

        // Check sourceType
        if (criteria.sourceType !== undefined && event.source?.type !== criteria.sourceType) {
          return false
        }

        // Check timestamp
        if (criteria.since !== undefined && event.timestamp < criteria.since) {
          return false
        }

        // Apply custom predicate if provided
        if (criteria.predicate && !criteria.predicate(event as BaseEvent<T>)) {
          return false
        }

        return true
      })
    )
  }

  /**
   * Get events filtered by category
   */
  getEventsByCategory$<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category })
  }

  /**
   * Get events filtered by type
   */
  getEventsByType$<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ type })
  }

  /**
   * Get events filtered by category and type
   */
  getEventsByCategoryAndType$<T = unknown>(category: EventCategory, type: string): Observable<BaseEvent<T>> {
    return this.getFilteredEvents$<T>({ category, type })
  }

  /**
   * Get events filtered by category (alias for getEventsByCategory$)
   */
  getEventsByCategory<T = unknown>(category: EventCategory): Observable<BaseEvent<T>> {
    return this.getEventsByCategory$<T>(category)
  }

  /**
   * Get events filtered by type (alias for getEventsByType$)
   */
  getEventsByType<T = unknown>(type: string): Observable<BaseEvent<T>> {
    return this.getEventsByType$<T>(type)
  }

  /**
   * Get events filtered by category and type (alias for getEventsByCategoryAndType$)
   */
  getEventsByCategoryAndType<T = unknown>(category: EventCategory, type: string): Observable<BaseEvent<T>> {
    return this.getEventsByCategoryAndType$<T>(category, type)
  }

  /**
   * Publish an event to all subscribers
   */
  publish(event: BaseEvent): void {
    // Add timestamp if not present
    if (!event.timestamp) {
      event.timestamp = Date.now()
    }

    // Publish to subscribers
    this.eventSubject.next(event)

    // Forward to all renderer processes
    this.forwardToRenderers(event)
  }

  /**
   * Forward event to all connected renderer processes
   */
  private forwardToRenderers(event: BaseEvent): void {
    // Send to each window that's still valid
    for (const webContents of this.rendererWindows) {
      if (!webContents.isDestroyed()) {
        webContents.send('zelan:event', event)
      }
    }
  }

  /**
   * Get recent events with filtering options
   */
  getRecentEvents(options: EventCacheOptions = {}): BaseEvent[] {
    return this.eventCache.getEvents(options)
  }

  /**
   * Get a reactive stream of recent events
   */
  recentEvents$(filterCriteria: EventFilterCriteria = {}): Observable<BaseEvent[]> {
    return this.eventCache.filteredEvents$(filterCriteria)
  }
}
