import { BehaviorSubject, Observable, Subject, Subscription } from 'rxjs'
import { shareReplay, takeUntil } from 'rxjs/operators'
import { v4 as uuidv4 } from 'uuid'

import { Event } from '@shared/types/events'
import {
  AdapterConnectedEvent,
  AdapterDisconnectedEvent,
  AdapterErrorEvent,
  AdapterReconnectingEvent
} from '@shared/core/events'

import { ServiceAdapter, AdapterConfig, AdapterStatus, AdapterStatusInfo } from '../interfaces'

/**
 * Base implementation for service adapters
 */
export abstract class BaseAdapter implements ServiceAdapter {
  /**
   * Subject to notify when the adapter is being disposed
   */
  protected readonly dispose$ = new Subject<void>()

  /**
   * Subject that emits events from this adapter
   */
  protected readonly eventSubject = new Subject<Event>()

  /**
   * Subject that emits status changes for this adapter
   */
  protected readonly statusSubject = new BehaviorSubject<AdapterStatusInfo>({
    status: AdapterStatus.DISCONNECTED,
    lastUpdated: Date.now()
  })

  /**
   * Shared observable of events from this adapter
   */
  protected readonly sharedEvents$: Observable<Event>

  /**
   * Active subscriptions
   */
  protected subscriptions: Subscription[] = []

  /**
   * Current adapter configuration
   */
  protected config: AdapterConfig

  /**
   * Stores if the adapter has been initialized
   */
  protected initialized = false

  /**
   * Get the adapter ID
   */
  public get id(): string {
    return this.config?.id
  }

  /**
   * Get the adapter type
   */
  public get type(): string {
    return this.config?.type
  }

  /**
   * Get the adapter name
   */
  public get name(): string {
    return this.config?.name
  }

  /**
   * Get the current adapter status
   */
  public get status(): AdapterStatus {
    return this.statusSubject.value.status
  }

  /**
   * Get extended status information
   */
  public get statusInfo(): AdapterStatusInfo {
    return this.statusSubject.value
  }

  /**
   * Observable of status changes
   */
  public get status$(): Observable<AdapterStatusInfo> {
    return this.statusSubject.asObservable()
  }

  /**
   * Observable of events from this adapter
   */
  public get events$(): Observable<Event> {
    return this.sharedEvents$
  }

  /**
   * Check if the adapter is connected
   */
  public get isConnected(): boolean {
    return this.status === AdapterStatus.CONNECTED
  }

  /**
   * Check if the adapter is enabled
   */
  public get isEnabled(): boolean {
    return this.config?.enabled === true
  }

  /**
   * Create a new base adapter
   *
   * @param defaultConfig Default configuration
   */
  constructor(defaultConfig?: AdapterConfig) {
    // Initialize with default config if provided
    this.config = defaultConfig || {
      id: uuidv4(),
      type: 'unknown',
      name: 'Unknown Adapter',
      enabled: false,
      autoConnect: false,
      autoReconnect: true,
      maxReconnectAttempts: 5,
      settings: {}
    }

    // Create shared events observable
    this.sharedEvents$ = this.eventSubject.pipe(takeUntil(this.dispose$), shareReplay(100))
  }

  /**
   * Initialize the adapter with configuration
   *
   * @param config Adapter configuration
   */
  public async initialize(config: AdapterConfig): Promise<void> {
    // Merge with existing config
    this.config = {
      ...this.config,
      ...config
    }

    // Update status
    this.updateStatus({
      status: AdapterStatus.DISCONNECTED,
      lastUpdated: Date.now()
    })

    // Mark as initialized
    this.initialized = true

    // Connect automatically if needed
    if (this.config.enabled && this.config.autoConnect) {
      await this.connect()
    }
  }

  /**
   * Connect to the service
   */
  public async connect(): Promise<void> {
    if (!this.initialized) {
      throw new Error('Adapter not initialized')
    }

    if (!this.isEnabled) {
      throw new Error('Adapter is disabled')
    }

    if (this.isConnected) {
      return
    }

    // Update status to connecting
    this.updateStatus({
      status: AdapterStatus.CONNECTING,
      lastUpdated: Date.now()
    })

    try {
      // Call the implementation-specific connection logic
      await this.connectImpl()

      // Update status to connected
      this.updateStatus({
        status: AdapterStatus.CONNECTED,
        lastUpdated: Date.now(),
        lastConnected: Date.now()
      })

      // Emit connection event
      this.emitEvent(new AdapterConnectedEvent(this.id))
    } catch (error) {
      // Update status to error
      this.updateStatus({
        status: AdapterStatus.ERROR,
        lastUpdated: Date.now(),
        error: error instanceof Error ? error.message : String(error)
      })

      // Emit error event
      this.emitEvent(new AdapterErrorEvent(this.id, String(error)))

      // Attempt reconnection if enabled
      if (this.config.autoReconnect) {
        this.handleReconnection()
      }

      throw error
    }
  }

  /**
   * Disconnect from the service
   */
  public async disconnect(): Promise<void> {
    if (!this.isConnected) {
      return
    }

    // Update status to disconnecting
    this.updateStatus({
      status: AdapterStatus.DISCONNECTED,
      lastUpdated: Date.now()
    })

    try {
      // Call the implementation-specific disconnection logic
      await this.disconnectImpl()

      // Emit disconnection event
      this.emitEvent(new AdapterDisconnectedEvent(this.id))
    } catch (error) {
      // Update status to error
      this.updateStatus({
        status: AdapterStatus.ERROR,
        lastUpdated: Date.now(),
        error: error instanceof Error ? error.message : String(error)
      })

      // Emit error event
      this.emitEvent(new AdapterErrorEvent(this.id, String(error)))

      throw error
    }
  }

  /**
   * Reconnect to the service
   */
  public async reconnect(): Promise<void> {
    await this.disconnect()
    await this.connect()
  }

  /**
   * Update the adapter configuration
   *
   * @param config New adapter configuration
   */
  public async updateConfig(config: Partial<AdapterConfig>): Promise<void> {
    // Merge with existing config
    this.config = {
      ...this.config,
      ...config
    }

    // Call implementation-specific config update
    await this.updateConfigImpl(config)
  }

  /**
   * Get the current adapter configuration
   */
  public getConfig(): AdapterConfig {
    return { ...this.config }
  }

  /**
   * Enable the adapter
   */
  public async enable(): Promise<void> {
    if (this.isEnabled) {
      return
    }

    await this.updateConfig({ enabled: true })

    if (this.config.autoConnect) {
      await this.connect()
    }
  }

  /**
   * Disable the adapter
   */
  public async disable(): Promise<void> {
    if (!this.isEnabled) {
      return
    }

    if (this.isConnected) {
      await this.disconnect()
    }

    await this.updateConfig({ enabled: false })
  }

  /**
   * Dispose of the adapter and clean up resources
   */
  public async dispose(): Promise<void> {
    // Disconnect if connected
    if (this.isConnected) {
      await this.disconnect()
    }

    // Notify about disposal
    this.dispose$.next()
    this.dispose$.complete()

    // Clean up subscriptions
    this.subscriptions.forEach((sub) => sub.unsubscribe())
    this.subscriptions = []

    // Call implementation-specific disposal
    await this.disposeImpl()
  }

  /**
   * Update the adapter status
   *
   * @param status New status information
   */
  protected updateStatus(status: Partial<AdapterStatusInfo>): void {
    // Merge with existing status
    const newStatus: AdapterStatusInfo = {
      ...this.statusSubject.value,
      ...status,
      lastUpdated: Date.now()
    }

    // Emit the new status
    this.statusSubject.next(newStatus)
  }

  /**
   * Emit an event from this adapter
   *
   * @param event Event to emit
   */
  protected emitEvent(event: Event): void {
    this.eventSubject.next(event)
  }

  /**
   * Handle reconnection logic
   */
  protected handleReconnection(): void {
    const maxAttempts = this.config.maxReconnectAttempts
    let attempts = 0

    // Update status to reconnecting
    this.updateStatus({
      status: AdapterStatus.RECONNECTING,
      lastUpdated: Date.now(),
      connectionAttempts: attempts
    })

    // Emit reconnecting event
    this.emitEvent(new AdapterReconnectingEvent(this.id))

    // Schedule reconnection attempt
    const attemptReconnect = async (): Promise<void> => {
      attempts++

      this.updateStatus({
        connectionAttempts: attempts
      })

      try {
        await this.connect()
      } catch (error) {
        // If we've reached the maximum attempts, stop trying
        if (maxAttempts !== -1 && attempts >= maxAttempts) {
          this.updateStatus({
            status: AdapterStatus.ERROR,
            lastUpdated: Date.now(),
            error: `Maximum reconnection attempts (${maxAttempts}) reached. Last error: ${error instanceof Error ? error.message : String(error)}`
          })
          return
        }

        // Exponential backoff
        const delay = Math.min(1000 * Math.pow(2, attempts), 30000)
        setTimeout(attemptReconnect, delay)
      }
    }

    // Start the reconnection process
    setTimeout(attemptReconnect, 1000)
  }

  /**
   * Implementation-specific connection logic
   */
  protected abstract connectImpl(): Promise<void>

  /**
   * Implementation-specific disconnection logic
   */
  protected abstract disconnectImpl(): Promise<void>

  /**
   * Implementation-specific configuration update logic
   *
   * @param config New configuration
   */
  protected abstract updateConfigImpl(config: Partial<AdapterConfig>): Promise<void>

  /**
   * Implementation-specific disposal logic
   */
  protected abstract disposeImpl(): Promise<void>
}
