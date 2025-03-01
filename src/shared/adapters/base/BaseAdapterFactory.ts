import { v4 as uuidv4 } from 'uuid'
import { AdapterFactory, AdapterInfo, AdapterConfig, ServiceAdapter } from '../interfaces'

/**
 * Base class for adapter factories
 */
export abstract class BaseAdapterFactory implements AdapterFactory {
  /**
   * Information about this adapter type
   */
  public readonly info: AdapterInfo

  /**
   * Create a new adapter factory
   *
   * @param info Information about this adapter type
   */
  constructor(info: AdapterInfo) {
    this.info = info
  }

  /**
   * Create a new adapter instance
   *
   * @param config Configuration for the adapter
   * @returns A new adapter instance
   */
  public abstract createAdapter(config: AdapterConfig): ServiceAdapter

  /**
   * Get default configuration for this adapter type
   *
   * @param id Optional ID for the adapter
   * @param name Optional name for the adapter
   * @returns Default configuration
   */
  public getDefaultConfig(id?: string, name?: string): AdapterConfig {
    return {
      id: id || `${this.info.type}-${uuidv4()}`,
      type: this.info.type,
      name: name || this.info.name,
      enabled: false,
      autoConnect: false,
      autoReconnect: true,
      maxReconnectAttempts: 5,
      settings: {}
    }
  }
}
