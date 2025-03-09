import { AdapterManager } from './AdapterManager'
import { SubscriptionManager } from '@s/utils/subscription-manager'
import { getLoggingService, ComponentLogger } from '@m/services/logging'
import { EventBus } from '@s/core/bus'
import { filter } from 'rxjs/operators'
import { AdapterEventType, EventCategory } from '@s/types/events'
import { ConfigStore } from '@s/core/config'

/**
 * Reconnection options
 */
export interface ReconnectionOptions {
  /** Maximum reconnection attempts before backing off to long interval */
  maxAttempts: number
  /** Initial delay before first retry (ms) */
  initialDelay: number 
  /** Maximum delay between retries (ms) */
  maxDelay: number
  /** Long interval delay after max attempts (ms) */
  longIntervalDelay: number
  /** Whether to auto-reset attempt count after successful reconnection */
  resetCountOnSuccess: boolean
}

/**
 * Default reconnection options
 */
const DEFAULT_OPTIONS: ReconnectionOptions = {
  maxAttempts: 5,
  initialDelay: 1000,
  maxDelay: 30000,
  longIntervalDelay: 60000,
  resetCountOnSuccess: true
}

/**
 * Tracked adapter state for reconnection
 */
interface ReconnectionState {
  attempts: number
  timer: NodeJS.Timeout | null
  lastAttempt: number
  adapterId: string
  adapterType: string
  adapterName: string
}

/**
 * Manages reconnection for adapters with exponential backoff
 * 
 * The ReconnectionManager listens for adapter connection errors and
 * automatically schedules reconnection attempts using an exponential
 * backoff strategy to avoid overwhelming services during outages.
 */
export class ReconnectionManager {
  private states = new Map<string, ReconnectionState>()
  private subscriptionManager = new SubscriptionManager()
  private logger: ComponentLogger
  private _options: ReconnectionOptions
  
  /**
   * Get current reconnection options
   */
  get options(): ReconnectionOptions {
    return { ...this._options }
  }
  
  /**
   * Create a new reconnection manager
   */
  constructor(
    private adapterManager: AdapterManager,
    private eventBus: EventBus,
    private configStore: ConfigStore
  ) {
    this.logger = getLoggingService().createLogger('ReconnectionManager')
    
    // Load options from config or use defaults
    this._options = {
      ...DEFAULT_OPTIONS,
      ...this.loadOptionsFromConfig()
    }
    
    this.logger.info('ReconnectionManager initialized', { options: this._options })
    
    // Subscribe to adapter error events
    this.setupErrorListener()
    
    // Subscribe to adapter status changes for successful connections
    this.setupStatusListener()
  }
  
  /**
   * Load reconnection options from config
   */
  private loadOptionsFromConfig(): Partial<ReconnectionOptions> {
    try {
      return this.configStore.get('settings.reconnection', {}) as Partial<ReconnectionOptions>
    } catch (error) {
      this.logger.error('Failed to load reconnection settings from config', {
        error: error instanceof Error ? error.message : String(error)
      })
      return {}
    }
  }
  
  /**
   * Setup listener for adapter error events
   */
  private setupErrorListener(): void {
    const subscription = this.eventBus.events$
      .pipe(
        filter(event => 
          event.category === EventCategory.ADAPTER && 
          event.type === AdapterEventType.ERROR
        )
      )
      .subscribe(event => {
        // Extract adapter info from event
        const adapterId = event.source.id
        
        // Check if the adapter exists and is enabled
        const adapter = this.adapterManager.getAdapter(adapterId)
        if (!adapter || !adapter.enabled) return
        
        this.logger.info(`Detected error in adapter ${adapterId}, scheduling reconnection`, {
          adapterId,
          adapterType: adapter.type,
          adapterName: adapter.name
        })
        
        // Schedule reconnection
        this.scheduleReconnect(adapterId, adapter.name, adapter.type)
      })
      
    this.subscriptionManager.add(subscription)
  }
  
  /**
   * Setup listener for successful connections to reset attempt counters
   */
  private setupStatusListener(): void {
    const subscription = this.eventBus.events$
      .pipe(
        filter(event => 
          event.category === EventCategory.ADAPTER && 
          event.type === AdapterEventType.CONNECTED
        )
      )
      .subscribe(event => {
        const adapterId = event.source.id
        
        // Reset reconnection attempts on successful connection if enabled
        if (this._options.resetCountOnSuccess) {
          const state = this.states.get(adapterId)
          if (state) {
            this.logger.debug(`Resetting reconnection attempts for ${adapterId}`, {
              adapterId,
              previousAttempts: state.attempts
            })
            
            state.attempts = 0
            state.lastAttempt = Date.now()
          }
        }
      })
      
    this.subscriptionManager.add(subscription)
  }
  
  /**
   * Schedule reconnection for an adapter with exponential backoff
   */
  public scheduleReconnect(
    adapterId: string, 
    adapterName: string,
    adapterType: string,
    forceNow = false
  ): void {
    // Get current state or create new one
    let state = this.states.get(adapterId)
    
    if (!state) {
      state = {
        attempts: 0,
        timer: null,
        lastAttempt: 0,
        adapterId,
        adapterName,
        adapterType
      }
      this.states.set(adapterId, state)
    }
    
    // Clear any existing timer
    this.clearTimer(adapterId)
    
    if (forceNow) {
      // Immediately attempt reconnection if forced
      this.attemptReconnection(adapterId)
      return
    }
    
    // Calculate delay based on attempts with exponential backoff
    let delay: number
    
    if (state.attempts >= this._options.maxAttempts) {
      // Use long interval delay after max attempts
      delay = this._options.longIntervalDelay
      this.logger.info(`Maximum reconnection attempts reached for ${adapterName} (${adapterId}). Using long interval.`, {
        adapterId,
        attempts: state.attempts,
        delay
      })
    } else {
      // Calculate exponential backoff: initialDelay * 2^attempts
      delay = Math.min(
        this._options.initialDelay * Math.pow(2, state.attempts),
        this._options.maxDelay
      )
      
      this.logger.info(`Scheduling reconnection for ${adapterName} (${adapterId})`, {
        adapterId,
        attempts: state.attempts,
        delay
      })
    }
    
    // Schedule reconnection
    state.timer = setTimeout(() => {
      this.attemptReconnection(adapterId)
    }, delay)
  }
  
  /**
   * Attempt to reconnect an adapter
   */
  private async attemptReconnection(adapterId: string): Promise<void> {
    const state = this.states.get(adapterId)
    if (!state) return
    
    // Get adapter
    const adapter = this.adapterManager.getAdapter(adapterId)
    if (!adapter || !adapter.enabled) {
      // Adapter no longer exists or is disabled, clean up state
      this.clearState(adapterId)
      return
    }
    
    // Increment attempt counter
    state.attempts++
    state.lastAttempt = Date.now()
    
    this.logger.info(`Attempting reconnection for ${adapter.name} (${adapterId})`, {
      adapterId,
      attempt: state.attempts,
      maxAttempts: this._options.maxAttempts
    })
    
    try {
      // Attempt reconnection
      await adapter.reconnect()
      
      this.logger.info(`Successfully reconnected ${adapter.name} (${adapterId})`, {
        adapterId
      })
      
      // Reset attempts if successful and reset is enabled
      if (this._options.resetCountOnSuccess) {
        state.attempts = 0
      }
    } catch (error) {
      this.logger.error(`Failed to reconnect ${adapter.name} (${adapterId})`, {
        error: error instanceof Error ? error.message : String(error),
        adapterId,
        attempt: state.attempts,
        maxAttempts: this._options.maxAttempts
      })
      
      // Schedule next attempt
      this.scheduleReconnect(adapterId, adapter.name, adapter.type)
    }
  }
  
  /**
   * Clear any pending reconnection timer for an adapter
   */
  private clearTimer(adapterId: string): void {
    const state = this.states.get(adapterId)
    if (state && state.timer) {
      clearTimeout(state.timer)
      state.timer = null
    }
  }
  
  /**
   * Clear all state for an adapter
   */
  private clearState(adapterId: string): void {
    this.clearTimer(adapterId)
    this.states.delete(adapterId)
  }
  
  /**
   * Update reconnection options
   */
  public updateOptions(options: Partial<ReconnectionOptions>): void {
    this._options = {
      ...this._options,
      ...options
    }
    
    // Save to config
    this.configStore.set('settings.reconnection', this._options)
    
    this.logger.info('Updated reconnection options', { options: this._options })
  }
  
  /**
   * Force immediate reconnection of an adapter
   */
  public reconnectNow(adapterId: string): Promise<void> {
    const adapter = this.adapterManager.getAdapter(adapterId)
    if (!adapter || !adapter.enabled) {
      throw new Error(`Adapter ${adapterId} not found or disabled`)
    }
    
    this.clearTimer(adapterId)
    return adapter.reconnect()
  }
  
  /**
   * Force immediate reconnection of all enabled adapters
   */
  public reconnectAllNow(): Promise<void> {
    return this.adapterManager.reconnectAll()
  }
  
  /**
   * Cancel pending reconnections for an adapter
   */
  public cancelReconnection(adapterId: string): void {
    this.clearTimer(adapterId)
    this.logger.info(`Cancelled reconnection for ${adapterId}`, { adapterId })
  }
  
  /**
   * Get current reconnection state for an adapter
   */
  public getReconnectionState(adapterId: string): {
    pending: boolean,
    attempts: number,
    lastAttempt: number
  } | null {
    const state = this.states.get(adapterId)
    if (!state) return null
    
    return {
      pending: state.timer !== null,
      attempts: state.attempts,
      lastAttempt: state.lastAttempt
    }
  }
  
  /**
   * Dispose of all resources
   */
  public dispose(): void {
    // Clean up subscriptions
    this.subscriptionManager.unsubscribeAll()
    
    // Clear all timers
    for (const adapterId of this.states.keys()) {
      this.clearTimer(adapterId)
    }
    
    this.states.clear()
    this.logger.info('ReconnectionManager disposed')
  }
}