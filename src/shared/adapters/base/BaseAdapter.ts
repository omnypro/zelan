import { BehaviorSubject, Observable, Subject } from 'rxjs'
import { AdapterConfig, ServiceAdapter } from '../interfaces/ServiceAdapter'
import { AdapterStatus, AdapterStatusInfo } from '../interfaces/AdapterStatus'
import { EventBus } from '@s/core/bus'
import { EventCategory, AdapterEventType } from '@s/types/events'
import { createEvent } from '@s/core/events'
import { isObject, isString, isBoolean } from '@s/utils/type-guards'

/**
 * Abstract base class for all service adapters
 */
export abstract class BaseAdapter implements ServiceAdapter {
  readonly id: string
  readonly type: string
  private _name: string
  readonly eventBus: EventBus

  private _enabled: boolean
  protected _options: Record<string, unknown>
  protected _status$ = new BehaviorSubject<AdapterStatusInfo>({
    status: AdapterStatus.DISCONNECTED,
    timestamp: Date.now()
  })

  // Cleanup subject for managing subscriptions
  protected destroy$ = new Subject<void>()

  constructor(
    id: string,
    type: string,
    name: string,
    options: Record<string, unknown>,
    eventBus: EventBus,
    enabled = true
  ) {
    this.id = id
    this.type = type
    this._name = name
    this._options = options
    this.eventBus = eventBus
    this._enabled = enabled
  }

  /**
   * Get the adapter name
   */
  get name(): string {
    return this._name
  }

  get status$(): Observable<AdapterStatusInfo> {
    return this._status$.asObservable()
  }

  get status(): AdapterStatusInfo {
    return this._status$.value
  }

  get enabled(): boolean {
    return this._enabled
  }

  get options(): Record<string, unknown> {
    return { ...this._options }
  }

  /**
   * Initialize the adapter
   */
  async initialize(): Promise<void> {
    this.updateStatus(AdapterStatus.DISCONNECTED)
    if (this.enabled) {
      await this.connect()
    }
  }

  /**
   * Connect to the service
   */
  async connect(): Promise<void> {
    if (!this.enabled) {
      throw new Error(`Adapter ${this.name} (${this.id}) is disabled`)
    }

    try {
      this.updateStatus(AdapterStatus.CONNECTING)
      await this.connectImplementation()
      this.updateStatus(AdapterStatus.CONNECTED)
    } catch (error) {
      this.updateStatus(AdapterStatus.ERROR, 'Connection failed', error as Error)
      throw error
    }
  }

  /**
   * Disconnect from the service
   */
  async disconnect(): Promise<void> {
    try {
      await this.disconnectImplementation()
      this.updateStatus(AdapterStatus.DISCONNECTED)
    } catch (error) {
      this.updateStatus(AdapterStatus.ERROR, 'Disconnection failed', error as Error)
      throw error
    }
  }

  /**
   * Reconnect to the service
   */
  async reconnect(): Promise<void> {
    if (!this.enabled) {
      throw new Error(`Adapter ${this.name} (${this.id}) is disabled`)
    }

    try {
      this.updateStatus(AdapterStatus.RECONNECTING)
      await this.disconnectImplementation()
      await this.connectImplementation()
      this.updateStatus(AdapterStatus.CONNECTED)
    } catch (error) {
      this.updateStatus(AdapterStatus.ERROR, 'Reconnection failed', error as Error)
      throw error
    }
  }

  /**
   * Update adapter configuration
   * @param config Updated configuration
   */
  async updateConfig(config: Partial<AdapterConfig>): Promise<void> {
    const wasEnabled = this._enabled

    // Validate the config before applying it
    this.validateConfigUpdate(config)

    // Update properties
    if (config.name !== undefined) {
      this._name = config.name
    }

    if (config.enabled !== undefined) {
      this._enabled = config.enabled
    }

    if (config.options) {
      this._options = {
        ...this._options,
        ...config.options
      }
    }

    // Handle enabled/disabled state changes
    if (wasEnabled && !this.enabled) {
      await this.disconnect()
    } else if (!wasEnabled && this.enabled) {
      await this.connect()
    } else if (this.enabled && (config.options || Object.keys(config).length > 0)) {
      // If options changed and we're enabled, reconnect
      await this.reconnect()
    }
  }

  /**
   * Dispose of resources used by the adapter
   */
  async dispose(): Promise<void> {
    // Signal all subscriptions to complete
    this.destroy$.next()
    this.destroy$.complete()

    // Disconnect if connected
    if (this._status$.value.status !== AdapterStatus.DISCONNECTED) {
      await this.disconnect()
    }

    // Cleanup implementation-specific resources
    await this.disposeImplementation()
  }

  /**
   * Helper method to create an adapter event
   */
  protected createAdapterEvent(eventType: string, data: unknown) {
    return createEvent(EventCategory.ADAPTER, eventType, data, this.id, this.name, this.type)
  }

  /**
   * Validate a configuration update
   * @throws Error if the configuration is invalid
   */
  protected validateConfigUpdate(config: Partial<AdapterConfig>): void {
    // Basic validation for common properties
    if (config.name !== undefined && !isString(config.name)) {
      throw new Error(`Invalid adapter name: ${config.name}, expected string`)
    }

    if (config.enabled !== undefined && !isBoolean(config.enabled)) {
      throw new Error(`Invalid adapter enabled value: ${config.enabled}, expected boolean`)
    }

    if (config.options !== undefined && !isObject(config.options)) {
      throw new Error(`Invalid adapter options: ${config.options}, expected object`)
    }

    // Derived classes can override to add more specific validation
  }

  /**
   * Update the adapter status and emit events
   */
  protected updateStatus(status: AdapterStatus, message?: string, error?: Error): void {
    const statusInfo: AdapterStatusInfo = {
      status,
      message,
      error,
      timestamp: Date.now()
    }

    this._status$.next(statusInfo)

    // Create the event data once
    const eventData = {
      status: statusInfo.status,
      message: statusInfo.message,
      timestamp: statusInfo.timestamp,
      error: error
        ? {
            message: error.message,
            stack: error.stack
          }
        : undefined
    }

    // Publish a single status event with all information
    this.eventBus.publish(this.createAdapterEvent(AdapterEventType.STATUS, eventData))
  }

  /**
   * Implementation-specific connect logic
   */
  protected abstract connectImplementation(): Promise<void>

  /**
   * Implementation-specific disconnect logic
   */
  protected abstract disconnectImplementation(): Promise<void>

  /**
   * Implementation-specific resource cleanup
   */
  protected abstract disposeImplementation(): Promise<void>
}
