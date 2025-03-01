import { BaseAdapter } from '@shared/adapters/base'
import { AdapterConfig, AdapterStatus } from '@shared/adapters/interfaces'
import { SystemInfoEvent } from '@shared/core/events'

/**
 * Settings for the test adapter
 */
export interface TestAdapterSettings {
  /**
   * Interval in milliseconds between generated events
   */
  eventInterval: number

  /**
   * Types of events to generate
   */
  eventTypes: string[]

  /**
   * Simulate connection failures
   */
  simulateFailures: boolean

  /**
   * Failure rate (0-1)
   */
  failureRate: number
}

/**
 * Default settings for the test adapter
 */
export const DEFAULT_TEST_SETTINGS: TestAdapterSettings = {
  eventInterval: 5000,
  eventTypes: ['info', 'warning', 'error'],
  simulateFailures: false,
  failureRate: 0.1
}

/**
 * Adapter that generates test events
 */
export class TestAdapter extends BaseAdapter {
  /**
   * The timer interval for generating events
   */
  private eventTimer: NodeJS.Timeout | null = null

  /**
   * Get the typed settings for this adapter
   */
  private get settings(): TestAdapterSettings {
    const settings = this.config.settings as Record<string, unknown>
    return {
      eventInterval:
        typeof settings.eventInterval === 'number'
          ? settings.eventInterval
          : DEFAULT_TEST_SETTINGS.eventInterval,
      eventTypes: Array.isArray(settings.eventTypes)
        ? (settings.eventTypes as string[])
        : DEFAULT_TEST_SETTINGS.eventTypes,
      simulateFailures:
        typeof settings.simulateFailures === 'boolean'
          ? settings.simulateFailures
          : DEFAULT_TEST_SETTINGS.simulateFailures,
      failureRate:
        typeof settings.failureRate === 'number'
          ? settings.failureRate
          : DEFAULT_TEST_SETTINGS.failureRate
    }
  }

  /**
   * Implementation of the connection logic
   */
  protected async connectImpl(): Promise<void> {
    // Simulate a connection delay
    await new Promise((resolve) => setTimeout(resolve, 1000))

    // Simulate random connection failures if enabled
    if (this.settings.simulateFailures && Math.random() < this.settings.failureRate) {
      throw new Error('Simulated connection failure')
    }

    // Start generating events
    this.startEventGeneration()
  }

  /**
   * Implementation of the disconnection logic
   */
  protected async disconnectImpl(): Promise<void> {
    // Stop generating events
    this.stopEventGeneration()

    // Simulate a disconnection delay
    await new Promise((resolve) => setTimeout(resolve, 500))
  }

  /**
   * Implementation of the configuration update logic
   */
  protected async updateConfigImpl(config: Partial<AdapterConfig>): Promise<void> {
    // Restart event generation if we're connected and the interval changed
    if (
      this.isConnected &&
      config.settings &&
      (config.settings as Partial<TestAdapterSettings>).eventInterval !== undefined
    ) {
      this.stopEventGeneration()
      this.startEventGeneration()
    }
  }

  /**
   * Implementation of the disposal logic
   */
  protected async disposeImpl(): Promise<void> {
    this.stopEventGeneration()
  }

  /**
   * Start generating test events
   */
  private startEventGeneration(): void {
    // Stop any existing timer
    this.stopEventGeneration()

    // Start a new timer
    this.eventTimer = setInterval(() => {
      this.generateRandomEvent()
    }, this.settings.eventInterval)
  }

  /**
   * Stop generating test events
   */
  private stopEventGeneration(): void {
    if (this.eventTimer) {
      clearInterval(this.eventTimer)
      this.eventTimer = null
    }
  }

  /**
   * Generate a random test event
   */
  private generateRandomEvent(): void {
    const { eventTypes } = this.settings

    // Select a random event type
    const eventType = eventTypes[Math.floor(Math.random() * eventTypes.length)]

    // Create a test message
    const message = `Test ${eventType} event at ${new Date().toISOString()}`

    // Emit a system info event
    this.emitEvent(new SystemInfoEvent(message))

    // Simulate random failures during operation if enabled
    if (this.settings.simulateFailures && Math.random() < this.settings.failureRate / 10) {
      this.updateStatus({
        status: eventType === 'error' ? AdapterStatus.ERROR : AdapterStatus.CONNECTED,
        error: eventType === 'error' ? 'Simulated random error during operation' : undefined
      })
    }
  }
}
