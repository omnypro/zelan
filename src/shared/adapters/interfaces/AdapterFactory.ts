import { ServiceAdapter, AdapterConfig } from './ServiceAdapter'

/**
 * Information about an adapter type
 */
export interface AdapterInfo {
  /**
   * Unique type identifier for this adapter
   */
  type: string

  /**
   * User-friendly name for this adapter type
   */
  name: string

  /**
   * Short description of the adapter
   */
  description: string

  /**
   * Version of the adapter
   */
  version: string

  /**
   * Author of the adapter
   */
  author: string

  /**
   * Whether this adapter requires authentication
   */
  requiresAuth: boolean

  /**
   * Schema for the adapter's settings
   */
  settingsSchema?: Record<string, unknown>
}

/**
 * Factory for creating adapter instances
 */
export interface AdapterFactory {
  /**
   * Information about this adapter type
   */
  readonly info: AdapterInfo

  /**
   * Create a new adapter instance
   *
   * @param config Configuration for the adapter
   * @returns A new adapter instance
   */
  createAdapter(config: AdapterConfig): ServiceAdapter

  /**
   * Get default configuration for this adapter type
   *
   * @param id Optional ID for the adapter
   * @param name Optional name for the adapter
   * @returns Default configuration
   */
  getDefaultConfig(id?: string, name?: string): AdapterConfig
}
