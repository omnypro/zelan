import { ElectronAPI } from '@electron-toolkit/preload'
import { Observable } from 'rxjs'
import { AppConfig, ConfigChangeEvent } from '../shared/core/config'
import type { AppRouter } from '../shared/trpc'
import type { inferRouterProxyClient } from '@trpc/client'

/**
 * Event API for the renderer process
 */
interface EventAPI {
  /**
   * Send an event to the main process (async)
   */
  sendEvent: (event: any) => void
  
  /**
   * Send an event to the main process (sync)
   */
  sendEventSync: (event: any) => any
  
  /**
   * Listen for events from the main process
   */
  onEvent: (callback: (event: any) => void) => () => void
}

/**
 * Configuration API for the renderer process
 */
interface ConfigAPI {
  /**
   * Get a value from the config store
   */
  get: <T>(key: string, defaultValue?: T) => Promise<T>
  
  /**
   * Set a value in the config store
   */
  set: <T>(key: string, value: T) => Promise<boolean>
  
  /**
   * Update multiple config values at once
   */
  update: (updates: Record<string, any>) => Promise<boolean>
  
  /**
   * Get the config file path
   */
  getPath: () => Promise<string | null>
  
  /**
   * Delete a key from the config store
   */
  delete: (key: string) => Promise<boolean>
  
  /**
   * Check if a key exists in the config store
   */
  has: (key: string) => Promise<boolean>
  
  /**
   * Get all configuration data
   */
  getAll: <T>() => Promise<T | null>
  
  /**
   * Get the full config and subscribe to updates
   */
  config$: (callback: (config: AppConfig) => void) => () => void
  
  /**
   * Get a specific config value and subscribe to updates
   */
  select$: <T>(path: string, defaultValue: T, callback: (value: T) => void) => () => void
  
  /**
   * Subscribe to all config change events
   */
  changes$: (callback: (event: ConfigChangeEvent) => void) => () => void
  
  /**
   * Subscribe to config change events for a specific path
   */
  changesFor$: (path: string, callback: (event: ConfigChangeEvent) => void) => () => void
}

/**
 * API exposed to the renderer process
 */
interface API {
  events: EventAPI
  config: ConfigAPI
}

declare global {
  interface Window {
    electron: ElectronAPI
    api: API
    trpc: inferRouterProxyClient<AppRouter>
  }
}