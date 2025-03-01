import { BaseAdapterFactory } from '@shared/adapters/base'
import { AdapterConfig, ServiceAdapter } from '@shared/adapters/interfaces'
import { TestAdapter, DEFAULT_TEST_SETTINGS } from './TestAdapter'

/**
 * Factory for creating test adapters
 */
export class TestAdapterFactory extends BaseAdapterFactory {
  /**
   * Create a new test adapter factory
   */
  constructor() {
    super({
      type: 'test',
      name: 'Test Adapter',
      description: 'Generates random test events for development and debugging',
      version: '1.0.0',
      author: 'Zelan',
      requiresAuth: false,
      settingsSchema: {
        type: 'object',
        properties: {
          eventInterval: {
            type: 'number',
            description: 'Interval in milliseconds between generated events',
            minimum: 100,
            maximum: 60000,
            default: DEFAULT_TEST_SETTINGS.eventInterval
          },
          eventTypes: {
            type: 'array',
            description: 'Types of events to generate',
            items: {
              type: 'string',
              enum: ['info', 'warning', 'error']
            },
            default: DEFAULT_TEST_SETTINGS.eventTypes
          },
          simulateFailures: {
            type: 'boolean',
            description: 'Simulate connection failures',
            default: DEFAULT_TEST_SETTINGS.simulateFailures
          },
          failureRate: {
            type: 'number',
            description: 'Failure rate (0-1)',
            minimum: 0,
            maximum: 1,
            default: DEFAULT_TEST_SETTINGS.failureRate
          }
        },
        required: ['eventInterval', 'eventTypes', 'simulateFailures', 'failureRate']
      }
    })
  }

  /**
   * Create a new test adapter
   *
   * @param config Configuration for the adapter
   * @returns A new test adapter
   */
  public createAdapter(config: AdapterConfig): ServiceAdapter {
    // Ensure settings has default values
    config.settings = {
      ...DEFAULT_TEST_SETTINGS,
      ...config.settings
    }

    return new TestAdapter(config)
  }

  /**
   * Get default configuration for a test adapter
   *
   * @param id Optional ID for the adapter
   * @param name Optional name for the adapter
   * @returns Default configuration
   */
  public override getDefaultConfig(id?: string, name?: string): AdapterConfig {
    const config = super.getDefaultConfig(id, name)

    // Add test-specific default settings
    config.settings = { ...DEFAULT_TEST_SETTINGS }

    return config
  }
}
