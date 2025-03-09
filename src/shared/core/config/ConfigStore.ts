import { app } from 'electron'
import * as fs from 'fs'
import * as path from 'path'
import { BehaviorSubject, Observable, Subject } from 'rxjs'
import { z } from 'zod'

// Define a simple console logger for ConfigStore when running in web context
interface ComponentLogger {
  info: (message: string, meta?: any) => void
  error: (message: string, meta?: any) => void
  warn: (message: string, meta?: any) => void
  debug: (message: string, meta?: any) => void
}

// Dynamic import for main process
let getLoggingService: () => { createLogger: (component: string) => ComponentLogger } | null = () => null

// Only create a console logger when in web context
// In the main process, we'll configure it properly during initialization
if (typeof app !== 'undefined') {
  getLoggingService = () => ({
    createLogger: (component: string) => ({
      info: (msg: string, meta?: any) => console.info(`[${component}] ${msg}`, meta || ''),
      error: (msg: string, meta?: any) => console.error(`[${component}] ${msg}`, meta || ''),
      warn: (msg: string, meta?: any) => console.warn(`[${component}] ${msg}`, meta || ''),
      debug: (msg: string, meta?: any) => console.debug(`[${component}] ${msg}`, meta || '')
    })
  })
}

/**
 * Event emitted when a configuration value changes
 */
export interface ConfigChangeEvent {
  key: string
  value: unknown
  previousValue?: unknown
  timestamp: number
}

/**
 * Adapter configuration schema
 */
const AdapterConfigSchema = z.object({
  id: z.string(),
  type: z.string(),
  name: z.string(),
  enabled: z.boolean(),
  options: z.record(z.unknown()).optional()
})

export type AdapterConfig = z.infer<typeof AdapterConfigSchema>

/**
 * Export AppConfig type for use elsewhere in the application
 */
export type AppConfig = {
  adapters: Record<string, AdapterConfig>
  settings: AppSettings
}

/**
 * Application settings schema
 */
const AppSettingsSchema = z.object({
  webSocketPort: z.number().default(8081),
  minimizeToTray: z.boolean().default(true),
  theme: z.enum(['light', 'dark', 'system']).default('system'),
  eventCacheSize: z.number().default(100),
  developerMode: z.boolean().default(false),
  logLevel: z.enum(['error', 'warn', 'info', 'debug']).default('info')
})

export type AppSettings = z.infer<typeof AppSettingsSchema>

// Define default config structure
const DEFAULT_CONFIG = {
  adapters: {},
  settings: AppSettingsSchema.parse({})
}

/**
 * Enhanced config store with reactive updates
 */
export class ConfigStore {
  private configPath: string
  private data: {
    adapters: Record<string, AdapterConfig>
    settings: AppSettings
  }

  private adaptersSubject: BehaviorSubject<Record<string, AdapterConfig>>
  private settingsSubject: BehaviorSubject<AppSettings>
  private changesSubject = new Subject<ConfigChangeEvent>()
  private logger!: ComponentLogger

  /**
   * Create a new config store
   */
  constructor() {
    // In renderer process, app might not be available
    const userDataPath = app ? app.getPath('userData') : ''
    this.configPath = path.join(userDataPath, 'config.json')

    // Initialize with defaults
    this.data = { ...DEFAULT_CONFIG }

    // Create subjects
    this.adaptersSubject = new BehaviorSubject<Record<string, AdapterConfig>>({})
    this.settingsSubject = new BehaviorSubject<AppSettings>(this.data.settings)

    // Initialize logger
    if (app) {
      const loggingService = getLoggingService()
      if (loggingService) {
        this.logger = loggingService.createLogger('ConfigStore')
      } else {
        // Fallback console logger
        this.logger = {
          info: (msg, meta) => console.info(`[ConfigStore] ${msg}`, meta),
          error: (msg, meta) => console.error(`[ConfigStore] ${msg}`, meta),
          warn: (msg, meta) => console.warn(`[ConfigStore] ${msg}`, meta),
          debug: (msg, meta) => console.debug(`[ConfigStore] ${msg}`, meta)
        }
      }

      // Load config if in main process where app is available
      this.loadConfig()
    } else {
      // Simple console logger for web context
      this.logger = {
        info: (msg, meta) => console.info(`[ConfigStore] ${msg}`, meta),
        error: (msg, meta) => console.error(`[ConfigStore] ${msg}`, meta),
        warn: (msg, meta) => console.warn(`[ConfigStore] ${msg}`, meta),
        debug: (msg, meta) => console.debug(`[ConfigStore] ${msg}`, meta)
      }
    }
  }

  /**
   * Safely get a nested property value from an object using a dot-separated path
   */
  private getValueAtPath<T>(obj: any, path: string[], defaultValue?: T): T {
    let current = obj

    for (const part of path) {
      if (current === undefined || current === null) {
        return defaultValue as T
      }
      current = current[part]
    }

    return current === undefined || current === null ? (defaultValue as T) : current
  }

  /**
   * Create and emit a change event
   */
  private emitChangeEvent(key: string, value: unknown, previousValue?: unknown): void {
    this.changesSubject.next({
      key,
      value,
      previousValue,
      timestamp: Date.now()
    })
  }

  /**
   * Load configuration from disk
   */
  private loadConfig(): void {
    try {
      if (!fs.existsSync(this.configPath)) {
        // If file doesn't exist, create it with defaults
        this.saveConfig()
        return
      }

      const fileContent = fs.readFileSync(this.configPath, 'utf8')
      const fileData = JSON.parse(fileContent)

      // Process adapters
      if (fileData.adapters && typeof fileData.adapters === 'object') {
        const validAdapters: Record<string, AdapterConfig> = {}

        for (const [id, adapter] of Object.entries(fileData.adapters)) {
          try {
            validAdapters[id] = AdapterConfigSchema.parse(adapter)
          } catch (err) {
            this.logger.error(`Invalid adapter config for ${id}`, {
              error: err instanceof Error ? err.message : String(err)
            })
          }
        }

        this.data.adapters = validAdapters
        this.adaptersSubject.next({ ...validAdapters })
      }

      // Process settings
      if (fileData.settings) {
        try {
          this.data.settings = AppSettingsSchema.parse({
            ...DEFAULT_CONFIG.settings,
            ...fileData.settings
          })
          this.settingsSubject.next({ ...this.data.settings })
        } catch (err) {
          this.logger.error('Invalid settings', {
            error: err instanceof Error ? err.message : String(err)
          })
        }
      }
    } catch (err) {
      this.logger.error('Error loading config', {
        error: err instanceof Error ? err.message : String(err)
      })
      // Use defaults on error
      this.data = { ...DEFAULT_CONFIG }
      this.saveConfig()
    }
  }

  /**
   * Save configuration to disk
   */
  private saveConfig(): void {
    try {
      // Make sure the directory exists
      const dir = path.dirname(this.configPath)
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true })
      }

      // Write the config file
      fs.writeFileSync(this.configPath, JSON.stringify(this.data, null, 2))
    } catch (err) {
      this.logger.error('Error saving config', {
        error: err instanceof Error ? err.message : String(err)
      })
    }
  }

  /**
   * Get the file path of the config file
   */
  get fileName(): string {
    return this.configPath
  }

  /**
   * Adapter Methods
   */

  /**
   * Save an adapter configuration
   */
  saveAdapter(adapter: AdapterConfig): void {
    const previousValue = this.data.adapters[adapter.id]

    // Validate adapter against schema
    const validAdapter = AdapterConfigSchema.parse(adapter)

    // Update internal state
    this.data.adapters[adapter.id] = validAdapter
    this.adaptersSubject.next({ ...this.data.adapters })

    // Emit change event
    this.emitChangeEvent(`adapters.${adapter.id}`, validAdapter, previousValue)

    // Persist to disk
    this.saveConfig()
  }

  /**
   * Delete an adapter configuration
   */
  deleteAdapter(id: string): void {
    if (this.data.adapters[id]) {
      const previousValue = this.data.adapters[id]

      // Update internal state
      delete this.data.adapters[id]
      this.adaptersSubject.next({ ...this.data.adapters })

      // Emit change event
      this.emitChangeEvent(`adapters.${id}`, undefined, previousValue)

      // Persist to disk
      this.saveConfig()
    }
  }

  /**
   * Get a specific adapter configuration by ID
   */
  getAdapter(id: string): AdapterConfig | undefined {
    return this.data.adapters[id]
  }

  /**
   * Get all adapter configurations
   */
  getAllAdapters(): Record<string, AdapterConfig> {
    return { ...this.data.adapters }
  }

  /**
   * Observable of all adapter configurations
   */
  adapters$(): Observable<Record<string, AdapterConfig>> {
    return this.adaptersSubject.asObservable()
  }

  /**
   * Settings Methods
   */

  /**
   * Update application settings
   */
  updateSettings(settings: Partial<AppSettings>): void {
    const previousValue = { ...this.data.settings }

    // Update with partial data
    const newSettings = {
      ...this.data.settings,
      ...settings
    }

    // Validate with schema
    const validSettings = AppSettingsSchema.parse(newSettings)

    // Update internal state
    this.data.settings = validSettings
    this.settingsSubject.next(validSettings)

    // Emit change event
    this.emitChangeEvent('settings', validSettings, previousValue)

    // Persist to disk
    this.saveConfig()
  }

  /**
   * Get all application settings
   */
  getSettings(): AppSettings {
    return { ...this.data.settings }
  }

  /**
   * Observable of all application settings
   */
  settings$(): Observable<AppSettings> {
    return this.settingsSubject.asObservable()
  }

  /**
   * Generic Config Methods
   */

  /**
   * Get a value from the configuration
   */
  get<T>(key: string, defaultValue?: T): T {
    const parts = key.split('.')
    return this.getValueAtPath<T>(this.data, parts, defaultValue)
  }

  /**
   * Update an adapter property or subproperty
   */
  private updateAdapterProperty(adapterId: string, propPath: string, value: unknown): void {
    const adapter = this.getAdapter(adapterId)
    if (!adapter) return

    let updatedAdapter: AdapterConfig

    // Handle different property paths for adapters
    if (propPath === 'options') {
      updatedAdapter = {
        ...adapter,
        options: value as Record<string, unknown>
      }
    } else if (propPath.startsWith('options.')) {
      const optionKey = propPath.substring(8)
      updatedAdapter = {
        ...adapter,
        options: {
          ...adapter.options,
          [optionKey]: value
        }
      }
    } else {
      updatedAdapter = {
        ...adapter,
        [propPath]: value
      }
    }

    this.saveAdapter(updatedAdapter)
  }

  /**
   * Set a value in the configuration
   */
  set<T>(key: string, value: T): void {
    const parts = key.split('.')
    const rootPart = parts[0]

    // Special handling for top-level paths
    if (rootPart === 'settings') {
      if (parts.length === 1) {
        // Full settings object
        this.updateSettings(value as unknown as Partial<AppSettings>)
      } else {
        // Individual setting
        const settingKey = parts[1] as keyof AppSettings
        const update = { [settingKey]: value } as unknown as Partial<AppSettings>
        this.updateSettings(update)
      }
      return
    } else if (rootPart === 'adapters') {
      if (parts.length > 1) {
        const adapterId = parts[1]
        if (parts.length === 2) {
          // Full adapter object
          this.saveAdapter(value as unknown as AdapterConfig)
        } else {
          // Adapter property
          const propPath = parts.slice(2).join('.')
          this.updateAdapterProperty(adapterId, propPath, value)
        }
        return
      }
    }

    // Generic path for other config sections
    const previousValue = this.get(key)

    // Build path to the property
    let current = this.data
    const lastIndex = parts.length - 1

    for (let i = 0; i < lastIndex; i++) {
      const part = parts[i]
      if (!(part in current)) {
        current[part] = {}
      }
      current = current[part]
    }

    // Set the value
    current[parts[lastIndex]] = value

    // Emit change event
    this.emitChangeEvent(key, value, previousValue)

    // Persist to disk
    this.saveConfig()
  }

  /**
   * Update multiple config values at once
   */
  update(updates: Record<string, unknown>): void {
    // Track if we need to save
    let needsSave = false
    // We'll use a timestamp for each specific change event instead of here

    // Batch all updates
    for (const [key, value] of Object.entries(updates)) {
      const previousValue = this.get(key)
      const parts = key.split('.')
      const rootPart = parts[0]

      // Handle special cases
      if (rootPart === 'settings' || rootPart === 'adapters') {
        // These have their own save methods
        this.set(key, value)
        continue
      }

      // Generic path
      let current = this.data
      const lastIndex = parts.length - 1

      for (let i = 0; i < lastIndex; i++) {
        const part = parts[i]
        if (!(part in current)) {
          current[part] = {}
        }
        current = current[part]
      }

      current[parts[lastIndex]] = value
      needsSave = true

      // Emit change event
      this.emitChangeEvent(key, value, previousValue)
    }

    // Save once if needed
    if (needsSave) {
      this.saveConfig()
    }
  }

  /**
   * Check if a key exists in the configuration
   */
  has(key: string): boolean {
    const parts = key.split('.')
    let current: any = this.data

    for (const part of parts) {
      if (current === undefined || current === null || !(part in current)) {
        return false
      }
      current = current[part]
    }

    return true
  }

  /**
   * Delete a key from the configuration
   */
  delete(key: string): void {
    const parts = key.split('.')

    if (parts.length === 0) {
      return
    }

    // Handle special paths
    if (parts[0] === 'adapters' && parts.length === 2) {
      this.deleteAdapter(parts[1])
      return
    }

    const previousValue = this.get(key)

    // Generic path
    let current = this.data
    const lastIndex = parts.length - 1

    for (let i = 0; i < lastIndex; i++) {
      const part = parts[i]
      if (!(part in current)) {
        return
      }
      current = current[part]
    }

    delete current[parts[lastIndex]]

    // Emit change event
    this.emitChangeEvent(key, undefined, previousValue)

    // Persist to disk
    this.saveConfig()
  }

  /**
   * Get all configuration as a single object
   * Note: This creates a deep copy to prevent modification of internal state
   */
  getAll(): Record<string, any> {
    return JSON.parse(JSON.stringify(this.data))
  }

  /**
   * Observable of all configuration changes
   */
  changes$(): Observable<ConfigChangeEvent> {
    return this.changesSubject.asObservable()
  }
}

/**
 * Singleton instance of the config store
 */
let configStoreInstance: ConfigStore | null = null

/**
 * Get the global config store instance
 */
export function getConfigStore(): ConfigStore {
  if (!configStoreInstance) {
    configStoreInstance = new ConfigStore()
  }
  return configStoreInstance
}
