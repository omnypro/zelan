import Store from 'electron-store';
import { z } from 'zod';

/**
 * Schema for global application configuration
 */
export const AppConfigSchema = z.object({
  // Application settings
  app: z.object({
    firstRun: z.boolean().default(true),
    theme: z.enum(['light', 'dark', 'system']).default('system'),
    logLevel: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
    startOnLogin: z.boolean().default(false),
    minimizeToTray: z.boolean().default(true),
  }).default({}),
  
  // WebSocket server configuration
  websocket: z.object({
    enabled: z.boolean().default(true),
    port: z.number().min(1024).max(65535).default(9090),
    path: z.string().default('/events'),
    pingInterval: z.number().min(1000).max(60000).default(30000),
    cors: z.object({
      enabled: z.boolean().default(true),
      origins: z.array(z.string()).default(['*']),
    }).default({}),
  }).default({}),
  
  // Event system configuration
  events: z.object({
    maxCachedEvents: z.number().min(10).max(10000).default(1000),
    logToConsole: z.boolean().default(false),
  }).default({}),
});

export type AppConfig = z.infer<typeof AppConfigSchema>;

/**
 * Configuration manager for global application settings
 * Uses electron-store with schema validation
 */
export class ConfigStore {
  private static instance: ConfigStore;
  private store: Store<AppConfig>;
  
  private constructor() {
    this.store = new Store({
      name: 'app-config',
      defaults: {
        app: {
          firstRun: true,
          theme: 'system',
          logLevel: 'info',
          startOnLogin: false,
          minimizeToTray: true,
        },
        websocket: {
          enabled: true,
          port: 9090,
          path: '/events',
          pingInterval: 30000,
          cors: {
            enabled: true,
            origins: ['*'],
          },
        },
        events: {
          maxCachedEvents: 1000,
          logToConsole: false,
        },
      },
    });
  }
  
  /**
   * Get singleton instance of ConfigStore
   */
  public static getInstance(): ConfigStore {
    if (!ConfigStore.instance) {
      ConfigStore.instance = new ConfigStore();
    }
    return ConfigStore.instance;
  }
  
  /**
   * Get the entire configuration
   */
  public getConfig(): AppConfig {
    return this.store.store;
  }
  
  /**
   * Update the application configuration
   */
  public updateConfig(config: Partial<AppConfig>): void {
    // Update each top-level section that exists in the config
    if (config.app) {
      const currentApp = this.get<AppConfig['app']>('app');
      this.set('app', { ...currentApp, ...config.app });
    }
    
    if (config.websocket) {
      const currentWebsocket = this.get<AppConfig['websocket']>('websocket');
      this.set('websocket', { ...currentWebsocket, ...config.websocket });
    }
    
    if (config.events) {
      const currentEvents = this.get<AppConfig['events']>('events');
      this.set('events', { ...currentEvents, ...config.events });
    }
  }
  
  /**
   * Set a configuration value at the specified path
   */
  public set<T>(key: string, value: T): void {
    this.store.set(key, value);
  }
  
  /**
   * Get a configuration value at the specified path
   */
  public get<T>(key: string): T {
    return this.store.get(key) as T;
  }
  
  /**
   * Check if a configuration key exists
   */
  public has(key: string): boolean {
    return this.store.has(key);
  }
  
  /**
   * Delete a configuration value
   */
  public delete(key: string): void {
    this.store.delete(key);
  }
  
  /**
   * Reset configuration to defaults
   */
  public reset(): void {
    this.store.clear();
  }
  
  /**
   * Get WebSocket server configuration
   */
  public getWebSocketConfig() {
    return this.get<AppConfig['websocket']>('websocket');
  }
  
  /**
   * Update WebSocket server configuration
   */
  public updateWebSocketConfig(config: Partial<AppConfig['websocket']>) {
    const currentConfig = this.getWebSocketConfig();
    this.set('websocket', {
      ...currentConfig,
      ...config,
    });
  }
  
  /**
   * Get event system configuration
   */
  public getEventConfig() {
    return this.get<AppConfig['events']>('events');
  }
  
  /**
   * Set the maximum number of cached events
   */
  public setMaxCachedEvents(count: number) {
    this.set('events.maxCachedEvents', count);
  }
  
  /**
   * Check if this is the first run of the application
   */
  public isFirstRun(): boolean {
    return this.get<boolean>('app.firstRun');
  }
  
  /**
   * Mark the application as having been run before
   */
  public markAsRun(): void {
    this.set('app.firstRun', false);
  }
  
  /**
   * Get the current theme setting
   */
  public getTheme(): 'light' | 'dark' | 'system' {
    return this.get<'light' | 'dark' | 'system'>('app.theme');
  }
  
  /**
   * Set the application theme
   */
  public setTheme(theme: 'light' | 'dark' | 'system'): void {
    this.set('app.theme', theme);
  }
}