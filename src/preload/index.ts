import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'
import { fromEvent } from 'rxjs'
import { map, filter, share } from 'rxjs/operators'
import { ConfigChangeEvent, AppConfig } from '@s/core/config'
import { trpcClient } from './trpc'

// Simple logger for preload process
const logger = {
  info: (...args: any[]) => console.log('[Preload]', ...args),
  error: (...args: any[]) => console.error('[Preload]', ...args),
  warn: (...args: any[]) => console.warn('[Preload]', ...args)
}

// IPC channels
const EVENT_CHANNEL = 'zelan:event'
const EVENT_SYNC_CHANNEL = 'zelan:event-sync'
const CONFIG_CHANGE_CHANNEL = 'zelan:config-change'

// Create an observable for config changes
const configChanges$ = fromEvent<[Electron.IpcRendererEvent, ConfigChangeEvent]>(
  ipcRenderer,
  CONFIG_CHANGE_CHANNEL
).pipe(
  map(([_, event]) => event),
  share()
)

// Event API for renderer process
const eventAPI = {
  /**
   * Send an event to the main process (async)
   */
  sendEvent: (event: any): void => {
    ipcRenderer.send(EVENT_CHANNEL, event)
  },

  /**
   * Send an event to the main process (sync)
   */
  sendEventSync: (event: any): any => {
    return ipcRenderer.sendSync(EVENT_SYNC_CHANNEL, event)
  },

  /**
   * Listen for events from the main process
   */
  onEvent: (callback: (event: any) => void): (() => void) => {
    const handler = (_: Electron.IpcRendererEvent, event: any) => {
      callback(event)
    }

    ipcRenderer.on(EVENT_CHANNEL, handler)

    // Return cleanup function
    return () => {
      ipcRenderer.removeListener(EVENT_CHANNEL, handler)
    }
  }
}

// Configuration API for renderer process
const configAPI = {
  /**
   * Get a value from the config store
   */
  get: <T>(key: string, defaultValue?: T): Promise<T> => {
    return ipcRenderer.invoke('config:get', key, defaultValue)
  },

  /**
   * Set a value in the config store
   */
  set: <T>(key: string, value: T): Promise<boolean> => {
    return ipcRenderer.invoke('config:set', key, value)
  },

  /**
   * Update multiple config values at once
   */
  update: (updates: Record<string, any>): Promise<boolean> => {
    return ipcRenderer.invoke('config:update', updates)
  },

  /**
   * Get the config file path
   */
  getPath: (): Promise<string | null> => {
    return ipcRenderer.invoke('config:path')
  },

  /**
   * Delete a key from the config store
   */
  delete: (key: string): Promise<boolean> => {
    return ipcRenderer.invoke('config:delete', key)
  },

  /**
   * Check if a key exists in the config store
   */
  has: (key: string): Promise<boolean> => {
    return ipcRenderer.invoke('config:has', key)
  },

  /**
   * Get all configuration data
   */
  getAll: <T>(): Promise<T | null> => {
    return ipcRenderer.invoke('config:getAll')
  },

  /**
   * Get the full config and subscribe to updates
   *
   * This returns a function that will call your callback whenever the config changes
   */
  config$: (callback: (config: AppConfig) => void): (() => void) => {
    // First, get the current config
    ipcRenderer
      .invoke('config:getAll')
      .then((config) => {
        // Send initial value
        callback(config as AppConfig)
      })
      .catch((err) => {
        logger.error('Error getting initial config:', err)
      })

    // Set up subscription to config changes
    const handler = () => {
      ipcRenderer
        .invoke('config:getAll')
        .then((updatedConfig) => {
          callback(updatedConfig as AppConfig)
        })
        .catch((err) => {
          logger.error('Error getting updated config:', err)
        })
    }

    // Subscribe to change events
    const subscription = configChanges$.subscribe(handler)

    // Return cleanup function
    return () => subscription.unsubscribe()
  },

  /**
   * Get a specific config value and subscribe to updates
   *
   * This returns a function that will call your callback whenever the config value changes
   */
  select$: <T>(path: string, defaultValue: T, callback: (value: T) => void): (() => void) => {
    // First, get the current value
    ipcRenderer
      .invoke('config:get', path, defaultValue)
      .then((value) => {
        // Send initial value
        callback(value as T)
      })
      .catch((err) => {
        logger.error(`Error getting initial value for ${path}:`, err)
      })

    // Set up subscription to matching config changes
    const handler = (event: ConfigChangeEvent) => {
      // For exact path matches, use the value directly
      if (event.key === path) {
        callback(event.value as T)
      } else if (event.key.startsWith(`${path}.`)) {
        // For nested path changes, get the full updated value
        ipcRenderer
          .invoke('config:get', path, defaultValue)
          .then((updatedValue) => {
            callback(updatedValue as T)
          })
          .catch((err) => {
            logger.error(`Error getting updated value for ${path}:`, err)
          })
      }
    }

    // Filter and subscribe to change events
    const changeSubscription = configChanges$
      .pipe(filter((event) => event.key === path || event.key.startsWith(`${path}.`)))
      .subscribe(handler)

    // Return cleanup function
    return () => changeSubscription.unsubscribe()
  },

  /**
   * Subscribe to all config change events
   *
   * This returns a function that will call your callback whenever any config changes
   */
  changes$: (callback: (event: ConfigChangeEvent) => void): (() => void) => {
    const subscription = configChanges$.subscribe(callback)
    return () => subscription.unsubscribe()
  },

  /**
   * Subscribe to config change events for a specific path
   *
   * This returns a function that will call your callback whenever matching config changes
   */
  changesFor$: (path: string, callback: (event: ConfigChangeEvent) => void): (() => void) => {
    const subscription = configChanges$
      .pipe(filter((event) => event.key === path || event.key.startsWith(`${path}.`)))
      .subscribe(callback)

    return () => subscription.unsubscribe()
  }
}

// Use `contextBridge` APIs to expose Electron APIs to
// renderer only if context isolation is enabled, otherwise
// just add to the DOM global.
if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', {
      events: eventAPI,
      config: configAPI
    })
    logger.info('Exposing tRPC client to main world:', !!trpcClient)
    contextBridge.exposeInMainWorld('trpc', trpcClient)
  } catch (error) {
    logger.error('Error exposing tRPC client:', error)
  }
} else {
  // @ts-ignore (define in dts)
  window.electron = electronAPI
  // @ts-ignore (define in dts)
  window.api = {
    events: eventAPI,
    config: configAPI
  }
  // @ts-ignore (define in dts)
  window.trpc = trpcClient
}
