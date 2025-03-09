import { BaseAdapter } from '@s/adapters/base'
import { EventBus } from '@s/core/bus'
import { EventCategory } from '@s/types/events'
import { createEvent } from '@s/core/events'
import { AdapterStatus } from '@s/adapters/interfaces/AdapterStatus'
import { AdapterConfig } from '@s/adapters/interfaces/ServiceAdapter'
import { isNumber, isBoolean, isStringArray, createObjectValidator } from '@s/utils/type-guards'
import { getLoggingService, ComponentLogger } from '@m/services/logging'

/**
 * Test adapter options
 */
export interface TestAdapterOptions {
  eventInterval: number
  simulateErrors: boolean
  eventTypes: string[]
}

/**
 * Default options for the test adapter
 */
const DEFAULT_OPTIONS: TestAdapterOptions = {
  eventInterval: 5000,
  simulateErrors: false,
  eventTypes: ['message', 'follow', 'subscription']
}

/**
 * Test adapter for demonstrations and testing
 */
export class TestAdapter extends BaseAdapter {
  private intervalId?: NodeJS.Timeout
  private eventCount = 0
  private logger: ComponentLogger

  constructor(
    id: string,
    name: string,
    options: Partial<TestAdapterOptions>,
    eventBus: EventBus,
    enabled = true
  ) {
    super(id, 'test', name, { ...DEFAULT_OPTIONS, ...options }, eventBus, enabled)
    this.logger = getLoggingService().createLogger(`TestAdapter:${id}`)
  }

  protected async connectImplementation(): Promise<void> {
    // Simulate connection delay
    await new Promise((resolve) => setTimeout(resolve, 500))

    // Start generating events
    this.startEventGeneration()
  }

  protected async disconnectImplementation(): Promise<void> {
    // Stop generating events
    this.stopEventGeneration()

    // Simulate disconnection delay
    await new Promise((resolve) => setTimeout(resolve, 300))
  }

  protected async disposeImplementation(): Promise<void> {
    this.stopEventGeneration()
  }

  /**
   * Validate a configuration update specifically for Test adapter
   * @throws Error if the configuration is invalid
   */
  protected override validateConfigUpdate(config: Partial<AdapterConfig>): void {
    // First validate using the base class implementation
    super.validateConfigUpdate(config)

    // Then perform Test-specific validation
    if (config.options) {
      // Validate event interval if provided
      if ('eventInterval' in config.options && !isNumber(config.options.eventInterval)) {
        throw new Error(`Invalid event interval: ${config.options.eventInterval}, expected number`)
      }

      // Validate simulate errors if provided
      if ('simulateErrors' in config.options && !isBoolean(config.options.simulateErrors)) {
        throw new Error(
          `Invalid simulateErrors value: ${config.options.simulateErrors}, expected boolean`
        )
      }

      // Validate event types if provided
      if ('eventTypes' in config.options && !isStringArray(config.options.eventTypes)) {
        throw new Error(
          `Invalid eventTypes: ${config.options.eventTypes}, expected array of strings`
        )
      }
    }
  }

  /**
   * Type guard for TestAdapterOptions
   */
  private static isTestAdapterOptions = createObjectValidator<TestAdapterOptions>({
    eventInterval: isNumber,
    simulateErrors: isBoolean,
    eventTypes: isStringArray
  })

  /**
   * Get the options with proper typing
   */
  private getTypedOptions(): TestAdapterOptions {
    // Use type guard to validate options at runtime
    if (!TestAdapter.isTestAdapterOptions(this.options)) {
      this.logger.warn('Invalid TestAdapter options, using defaults', this.options)
      return { ...DEFAULT_OPTIONS }
    }
    return this.options
  }

  /**
   * Start generating test events
   */
  private startEventGeneration(): void {
    const options = this.getTypedOptions()

    // Clear any existing interval
    this.stopEventGeneration()

    // Set up the interval to generate events
    this.intervalId = setInterval(() => {
      this.generateTestEvent()

      // Simulate random errors if enabled
      if (options.simulateErrors && Math.random() < 0.1) {
        this.updateStatus(
          AdapterStatus.ERROR,
          'Simulated random error',
          new Error('Test adapter simulated error')
        )

        // Automatically reconnect after a brief delay
        setTimeout(() => {
          this.reconnect().catch((error) => {
            this.logger.error('Failed to reconnect test adapter', error)
          })
        }, 3000)
      }
    }, options.eventInterval)
  }

  /**
   * Stop generating test events
   */
  private stopEventGeneration(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId)
      this.intervalId = undefined
    }
  }

  /**
   * Generate a random test event
   */
  private generateTestEvent(): void {
    const options = this.getTypedOptions()
    const eventTypes = options.eventTypes

    if (!eventTypes || eventTypes.length === 0) {
      return
    }

    this.eventCount++
    const eventType = eventTypes[Math.floor(Math.random() * eventTypes.length)]

    // Create event payload based on type
    let payload: Record<string, unknown>

    switch (eventType) {
      case 'message':
        payload = {
          username: `user${Math.floor(Math.random() * 1000)}`,
          message: `Test message ${this.eventCount} from test adapter`
        }
        break

      case 'follow':
        payload = {
          username: `user${Math.floor(Math.random() * 1000)}`,
          followDate: new Date().toISOString()
        }
        break

      case 'subscription':
        payload = {
          username: `user${Math.floor(Math.random() * 1000)}`,
          tier: Math.floor(Math.random() * 3) + 1,
          months: Math.floor(Math.random() * 24) + 1
        }
        break

      default:
        payload = {
          type: eventType,
          count: this.eventCount
        }
    }

    // Publish the event
    this.eventBus.publish(
      createEvent(
        EventCategory.ADAPTER,
        'data',
        {
          id: this.id,
          type: this.type,
          dataType: eventType,
          data: payload
        },
        this.id
      )
    )
  }
}
